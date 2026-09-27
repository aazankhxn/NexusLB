#![deny(unsafe_code)]

use ahash::AHashMap;
use bytes::Bytes;
use h2::server;
use http::{Request, Response};
use parking_lot::Mutex;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tracing::{debug, error, trace, warn};

use nexuslb_core::backend::{Backend, BackendConnectionGuard};
use nexuslb_core::types::BackendId;
use nexuslb_metrics::WorkerMetrics;
use nexuslb_observability::{AccessLogEntry, AccessLogger};
use nexuslb_router::{Router, SelectionContext};
use nexuslb_wasm::{FilterAction, FilterChain};

use crate::rate_limiter::RateLimiter;

/// Configurable HTTP/2 limits and timeout parameters (Issues #11 & #12)
#[derive(Debug, Clone)]
pub struct H2Config {
    pub max_concurrent_streams: u32,
    pub connect_timeout: Duration,
    pub stream_chunk_timeout: Duration,
    pub response_timeout: Duration,
}

impl Default for H2Config {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 128,
            connect_timeout: Duration::from_secs(5),
            stream_chunk_timeout: Duration::from_secs(30),
            response_timeout: Duration::from_secs(30),
        }
    }
}

/// Upstream HTTP/2 persistent connection pool for genuine multiplexing (Issue #2)
#[derive(Clone)]
pub struct H2ConnectionPool {
    pool: Arc<Mutex<AHashMap<BackendId, h2::client::SendRequest<Bytes>>>>,
    connect_timeout: Duration,
}

impl H2ConnectionPool {
    pub fn new(connect_timeout: Duration) -> Self {
        Self {
            pool: Arc::new(Mutex::new(AHashMap::new())),
            connect_timeout,
        }
    }

    pub fn invalidate(&self, backend_id: &BackendId) {
        let mut pool = self.pool.lock();
        pool.remove(backend_id);
    }

    /// Retrieve an existing ready upstream SendRequest handle or establish a persistent connection
    pub async fn get_or_connect(
        &self,
        backend: &Arc<Backend>,
    ) -> anyhow::Result<h2::client::SendRequest<Bytes>> {
        let backend_id = backend.id();

        // 1. Try reusing existing persistent connection handle
        let existing = {
            let pool = self.pool.lock();
            pool.get(&backend_id).cloned()
        };

        if let Some(client) = existing {
            // Verify the connection is not broken or closed
            if let Ok(Ok(ready_client)) = tokio::time::timeout(Duration::from_millis(50), client.ready()).await {
                return Ok(ready_client);
            }
            // Stale or closed connection: purge from pool
            self.invalidate(&backend_id);
        }

        // 2. Establish fresh TCP connection + H2 handshake
        let upstream_stream = tokio::time::timeout(
            self.connect_timeout,
            TcpStream::connect(backend.socket_addr()),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Upstream H2 connect timed out after {:?}", self.connect_timeout))??;
        let _ = upstream_stream.set_nodelay(true);

        let (client_h2, upstream_conn) = tokio::time::timeout(
            self.connect_timeout,
            h2::client::handshake(upstream_stream),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Upstream H2 handshake timed out after {:?}", self.connect_timeout))??;

        // Drive persistent upstream H2 connection in background
        tokio::spawn(async move {
            if let Err(e) = upstream_conn.await {
                debug!("Upstream persistent H2 connection terminated: {:?}", e);
            }
        });

        // Ensure connection is ready before registering in pool
        let ready_client = client_h2
            .ready()
            .await
            .map_err(|e| anyhow::anyhow!("Upstream H2 connection not ready: {:?}", e))?;

        let cached = ready_client.clone();
        {
            let mut pool = self.pool.lock();
            pool.insert(backend_id, cached);
        }

        Ok(ready_client)
    }
}

impl Default for H2ConnectionPool {
    fn default() -> Self {
        Self::new(Duration::from_secs(5))
    }
}

/// High-performance HTTP/2 and gRPC multiplexed stream proxy with per-stream routing (Issues #1, #2, #10)
pub struct H2Proxy;

impl H2Proxy {
    /// Accept HTTP/2 client connection and route each stream independently to backends
    pub async fn handle_connection<S>(
        client: S,
        client_addr: SocketAddr,
        router: Arc<Router>,
        h2_pool: Arc<H2ConnectionPool>,
        metrics: Arc<WorkerMetrics>,
        config: H2Config,
        access_logger: Arc<AccessLogger>,
        filter_chain: Arc<FilterChain>,
        rate_limiter: Arc<RateLimiter>,
        is_tls: bool,
    ) -> std::io::Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let mut connection = server::Builder::new()
            .max_concurrent_streams(config.max_concurrent_streams)
            .max_header_list_size(64 * 1024)
            .handshake(client)
            .await
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset,
                    format!("H2 handshake error: {:?}", e),
                )
            })?;

        trace!(client = %client_addr, "HTTP/2 connection handshake established");

        let stream_semaphore = Arc::new(tokio::sync::Semaphore::new(
            config.max_concurrent_streams as usize,
        ));

        let mut stream_count: u32 = 0;
        let mut window_start = Instant::now();
        const MAX_STREAMS_PER_SECOND: u32 = 500;

        while let Some(result) = connection.accept().await {
            let (req, mut respond) = match result {
                Ok(stream) => stream,
                Err(e) => {
                    debug!("H2 accept stream error: {:?}", e);
                    break;
                }
            };

            // Defense against HTTP/2 Rapid Reset attacks (CVE-2023-44487)
            if window_start.elapsed() >= Duration::from_secs(1) {
                stream_count = 0;
                window_start = Instant::now();
            }
            stream_count += 1;
            if stream_count > MAX_STREAMS_PER_SECOND {
                warn!(client = %client_addr, stream_rate = stream_count, "HTTP/2 stream creation rate limit exceeded (CVE-2023-44487 Rapid Reset defense), closing connection");
                respond.send_reset(h2::Reason::ENHANCE_YOUR_CALM);
                break;
            }

            // Per-stream rate limiting defense
            if !rate_limiter.check(Some(client_addr.ip())) {
                metrics.inc_dropped_connections();
                let response = Response::builder()
                    .status(429)
                    .header("content-type", "application/json")
                    .header("retry-after", "1")
                    .body(())
                    .unwrap();
                let mut send_stream = respond.send_response(response, false).ok();
                if let Some(ref mut s) = send_stream {
                    let _ = s.send_data(
                        Bytes::from_static(b"{\"error\":\"Too Many Requests\",\"message\":\"Rate limit exceeded\"}\n"),
                        true,
                    );
                }
                continue;
            }

            // Enforce concurrent stream limit
            let permit = match stream_semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    debug!("H2 connection exceeded max concurrent active streams");
                    let response = Response::builder()
                        .status(429)
                        .header("content-type", "text/plain")
                        .body(())
                        .unwrap();
                    let mut send_stream = respond.send_response(response, false).ok();
                    if let Some(ref mut s) = send_stream {
                        let _ = s.send_data(
                            Bytes::from_static(b"Too Many Requests (H2 Stream Limit)\n"),
                            true,
                        );
                    }
                    continue;
                }
            };

            let method = req.method().clone();
            let uri = req.uri().clone();
            let host_str = uri
                .host()
                .or_else(|| uri.authority().map(|a| a.host()))
                .or_else(|| req.headers().get("host").and_then(|h| h.to_str().ok()))
                .or_else(|| req.headers().get(":authority").and_then(|h| h.to_str().ok()));
            let path_str = uri.path();

            let mut m_str = method.as_str().to_string();
            let mut p_str = path_str.to_string();
            let mut headers_vec = Vec::with_capacity(req.headers().len() + 2);

            for (name, value) in req.headers() {
                let name_str = name.as_str();
                // Strip spoofed client headers that should only be injected by auth filters
                if name_str.eq_ignore_ascii_case("x-auth-subject")
                    || name_str.eq_ignore_ascii_case("x-user-id")
                    || name_str.eq_ignore_ascii_case("x-auth-status")
                {
                    continue;
                }
                if let Ok(v) = value.to_str() {
                    headers_vec.push((name_str.to_string(), v.to_string()));
                }
            }

            if !filter_chain.is_empty() {
                match filter_chain.execute_request(&mut m_str, &mut p_str, &mut headers_vec) {
                    FilterAction::StopAndReply { status, body, .. } => {
                        let response = Response::builder()
                            .status(status)
                            .header("content-type", "application/json")
                            .body(())
                            .unwrap();
                        let mut send_stream = respond.send_response(response, false).ok();
                        if let Some(ref mut s) = send_stream {
                            let _ = s.send_data(Bytes::from(body), true);
                        }
                        continue;
                    }
                    FilterAction::Drop => {
                        respond.send_reset(h2::Reason::CANCEL);
                        continue;
                    }
                    FilterAction::Continue => {}
                }
            }

            // Route EACH INDIVIDUAL STREAM using :authority, :path, method (respecting filter rewrites)
            let pool = router
                .route(host_str, &p_str, Some(&m_str), None, None)
                .map(|(_route, pool)| pool)
                .or_else(|| router.default_pool())
                .or_else(|| router.pools().values().next());

            let pool = match pool {
                Some(p) => p,
                None => {
                    metrics.inc_backend_errors();
                    let response = Response::builder()
                        .status(503)
                        .header("content-type", "application/json")
                        .body(())
                        .unwrap();
                    let mut send_stream = respond.send_response(response, false).ok();
                    if let Some(ref mut s) = send_stream {
                        let _ = s.send_data(
                            Bytes::from_static(
                                b"{\"error\":\"Service Unavailable\",\"message\":\"No backend pool configured\"}\n",
                            ),
                            true,
                        );
                    }
                    continue;
                }
            };

            let ctx = SelectionContext::with_ip(client_addr.ip());
            let backend = match pool.select(&ctx) {
                Some(b) => b,
                None => {
                    metrics.inc_backend_errors();
                    let response = Response::builder()
                        .status(503)
                        .header("content-type", "application/json")
                        .body(())
                        .unwrap();
                    let mut send_stream = respond.send_response(response, false).ok();
                    if let Some(ref mut s) = send_stream {
                        let _ = s.send_data(
                            Bytes::from_static(
                                b"{\"error\":\"Service Unavailable\",\"message\":\"No healthy backend available in pool\"}\n",
                            ),
                            true,
                        );
                    }
                    continue;
                }
            };

            // Acquire atomic connection reservation on selected backend
            let conn_guard = match BackendConnectionGuard::try_acquire(backend.clone()) {
                Some(g) => g,
                None => {
                    // Backend max_connections saturated
                    metrics.inc_backend_errors();
                    let response = Response::builder()
                        .status(503)
                        .header("content-type", "text/plain")
                        .body(())
                        .unwrap();
                    let mut send_stream = respond.send_response(response, false).ok();
                    if let Some(ref mut s) = send_stream {
                        let _ = s.send_data(Bytes::from_static(b"Backend Connection Limit Reached\n"), true);
                    }
                    continue;
                }
            };

            let metrics = metrics.clone();
            let h2_pool = h2_pool.clone();
            let config = config.clone();
            let access_logger = access_logger.clone();

            tokio::spawn(async move {
                let _permit = permit;
                let _guard = conn_guard;
                let start = Instant::now();
                metrics.inc_requests();
                metrics.inc_backend_requests();
                backend.stats().inc_requests();

                if let Err(e) = Self::proxy_stream(
                    req,
                    &mut respond,
                    backend.clone(),
                    h2_pool,
                    &config,
                    m_str.clone(),
                    p_str.clone(),
                    headers_vec,
                    client_addr.ip(),
                    is_tls,
                    access_logger,
                    start,
                )
                .await
                {
                    error!(error = %e, backend = %backend.name(), "H2 stream proxy error");
                    backend.stats().record_error();
                    metrics.inc_backend_errors();
                    let response = Response::builder()
                        .status(502)
                        .header("content-type", "text/plain")
                        .body(())
                        .unwrap();
                    let mut send_stream = respond.send_response(response, false).ok();
                    if let Some(ref mut s) = send_stream {
                        let _ = s.send_data(Bytes::from_static(b"Bad Gateway\n"), true);
                    }
                }
            });
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn proxy_stream(
        client_req: Request<h2::RecvStream>,
        client_respond: &mut server::SendResponse<Bytes>,
        backend: Arc<Backend>,
        h2_pool: Arc<H2ConnectionPool>,
        config: &H2Config,
        method_str: String,
        path_str: String,
        headers_vec: Vec<(String, String)>,
        client_ip: std::net::IpAddr,
        is_tls: bool,
        access_logger: Arc<AccessLogger>,
        start_time: Instant,
    ) -> anyhow::Result<()> {
        // Acquire or reuse persistent multiplexed upstream connection
        let client_h2 = h2_pool.get_or_connect(&backend).await?;

        // Wait for capacity to open stream on the multiplexed connection
        let mut ready_client = client_h2.ready().await.map_err(|e| {
            h2_pool.invalidate(&backend.id());
            anyhow::anyhow!("Upstream H2 connection capacity error: {:?}", e)
        })?;

        // Forward request headers
        let (parts, mut body_recv) = client_req.into_parts();
        let mut upstream_builder = Request::builder();
        if let Ok(m) = http::Method::from_bytes(method_str.as_bytes()) {
            upstream_builder = upstream_builder.method(m);
        } else {
            upstream_builder = upstream_builder.method(parts.method);
        }

        let mut uri_builder = http::uri::Builder::new();
        if let Some(scheme) = parts.uri.scheme() {
            uri_builder = uri_builder.scheme(scheme.clone());
        }
        if let Some(auth) = parts.uri.authority() {
            uri_builder = uri_builder.authority(auth.clone());
        }
        let pq = if let Some(q) = parts.uri.query() {
            format!("{}?{}", path_str, q)
        } else {
            path_str.clone()
        };
        if let Ok(uri) = uri_builder.path_and_query(pq).build() {
            upstream_builder = upstream_builder.uri(uri);
        } else {
            upstream_builder = upstream_builder.uri(parts.uri);
        }

        let mut upstream_req = upstream_builder.body(()).unwrap_or_else(|_| Request::new(()));
        let upstream_headers = upstream_req.headers_mut();

        for (k, v) in &headers_vec {
            // RFC 7540 §8.1.2.2: Filter connection-specific headers and pseudo-headers
            if k.starts_with(':')
                || k.eq_ignore_ascii_case("connection")
                || k.eq_ignore_ascii_case("keep-alive")
                || k.eq_ignore_ascii_case("proxy-connection")
                || k.eq_ignore_ascii_case("transfer-encoding")
                || k.eq_ignore_ascii_case("upgrade")
                || (k.eq_ignore_ascii_case("te") && v.as_str() != "trailers")
            {
                continue;
            }
            if let (Ok(name), Ok(val)) = (
                http::header::HeaderName::from_bytes(k.as_bytes()),
                http::header::HeaderValue::from_str(v),
            ) {
                upstream_headers.insert(name, val);
            }
        }

        let canonical_ip = crate::rate_limiter::canonicalize_ip(client_ip);
        let xff = canonical_ip.to_string();
        if let Ok(val) = http::header::HeaderValue::from_str(&xff) {
            upstream_headers.insert(http::header::HeaderName::from_static("x-forwarded-for"), val);
        }
        let proto = if is_tls { "https" } else { "http" };
        upstream_headers.insert(
            http::header::HeaderName::from_static("x-forwarded-proto"),
            http::header::HeaderValue::from_static(proto),
        );

        let is_end_of_stream = body_recv.is_end_stream();

        let (resp_fut, mut upstream_send_body) =
            ready_client.send_request(upstream_req, is_end_of_stream)?;

        // Forward client streaming data frames to upstream with chunk timeout
        if !is_end_of_stream {
            loop {
                let chunk = tokio::time::timeout(config.stream_chunk_timeout, body_recv.data())
                    .await
                    .map_err(|_| anyhow::anyhow!("Client H2 stream chunk timed out after {:?}", config.stream_chunk_timeout))?;
                match chunk {
                    Some(res) => {
                        let data = res?;
                        let is_eos = body_recv.is_end_stream();
                        let _ = body_recv.flow_control().release_capacity(data.len());
                        upstream_send_body.send_data(data, is_eos)?;
                        if is_eos {
                            break;
                        }
                    }
                    None => break,
                }
            }

            // Forward trailers if present (e.g. gRPC trailers)
            let trailers = tokio::time::timeout(config.stream_chunk_timeout, body_recv.trailers())
                .await
                .map_err(|_| anyhow::anyhow!("Client H2 stream trailers timed out"))??;
            if let Some(trailers) = trailers {
                upstream_send_body.send_trailers(trailers)?;
            }
        }

        // Await upstream response headers with response_timeout
        let upstream_resp = tokio::time::timeout(config.response_timeout, resp_fut)
            .await
            .map_err(|_| anyhow::anyhow!("Upstream H2 response timed out after {:?}", config.response_timeout))??;
        let status = upstream_resp.status();
        let (resp_parts, mut upstream_resp_body) = upstream_resp.into_parts();
        let client_resp = Response::from_parts(resp_parts, ());
        let resp_is_eos = upstream_resp_body.is_end_stream();

        let mut client_send_body = client_respond.send_response(client_resp, resp_is_eos)?;
        let mut total_sent = 0u64;

        // Forward upstream streaming response data frames to client with chunk timeout
        if !resp_is_eos {
            loop {
                let chunk = tokio::time::timeout(config.stream_chunk_timeout, upstream_resp_body.data())
                    .await
                    .map_err(|_| anyhow::anyhow!("Upstream H2 stream chunk timed out after {:?}", config.stream_chunk_timeout))?;
                match chunk {
                    Some(res) => {
                        let data = res?;
                        let is_eos = upstream_resp_body.is_end_stream();
                        let len = data.len();
                        let _ = upstream_resp_body
                            .flow_control()
                            .release_capacity(len);
                        client_send_body.send_data(data, is_eos)?;
                        total_sent += len as u64;
                        if is_eos {
                            break;
                        }
                    }
                    None => break,
                }
            }

            // Forward response trailers (e.g. grpc-status, grpc-message)
            let trailers = tokio::time::timeout(config.stream_chunk_timeout, upstream_resp_body.trailers())
                .await
                .map_err(|_| anyhow::anyhow!("Upstream H2 response trailers timed out"))??;
            if let Some(trailers) = trailers {
                client_send_body.send_trailers(trailers)?;
            }
        }

        let duration = start_time.elapsed();
        backend.stats().record_success(duration, 0, total_sent);
        access_logger.log(AccessLogEntry::new(
            client_ip,
            &method_str,
            &path_str,
            status.as_u16(),
            duration,
            backend.name(),
            total_sent,
        ));

        Ok(())
    }
}
