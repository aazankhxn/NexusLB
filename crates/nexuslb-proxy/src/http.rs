use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::trace;

use crate::retry::RetryPolicy;
use crate::tcp::TcpProxy;
use nexuslb_core::backend::Backend;
use nexuslb_metrics::WorkerMetrics;
use nexuslb_network::{BufferPool, ConnectionPool};

pub struct HttpProxy;

impl HttpProxy {
    /// Proxy an HTTP/1.1 or WebSocket request from client to selected backend with keep-alive
    #[allow(clippy::too_many_arguments)]
    pub async fn handle_connection(
        mut client: TcpStream,
        client_addr: SocketAddr,
        backend: Arc<Backend>,
        conn_pool: ConnectionPool,
        metrics: Arc<WorkerMetrics>,
        buffer_pool: BufferPool,
        _retry_policy: RetryPolicy,
        is_tls: bool,
        cache: Arc<nexuslb_cache::HttpCache>,
    ) -> std::io::Result<()> {
        let mut read_buf = buffer_pool.acquire();
        let mut resp_buf = buffer_pool.acquire();
        let mut req_bytes = Vec::with_capacity(1024);
        let mut total_read = 0;

        loop {
            // 1. Read until headers are fully available
            let (header_len, body_len) = loop {
                let mut headers = [httparse::EMPTY_HEADER; 64];
                let mut req = httparse::Request::new(&mut headers);

                match req.parse(&read_buf[..total_read]) {
                    Ok(httparse::Status::Complete(hlen)) => {
                        let mut cl = None;
                        for h in req.headers.iter() {
                            if h.name.eq_ignore_ascii_case("content-length") {
                                if let Ok(s) = std::str::from_utf8(h.value) {
                                    cl = s.trim().parse::<usize>().ok();
                                }
                                break;
                            }
                        }
                        break (hlen, cl.unwrap_or(0));
                    }
                    Ok(httparse::Status::Partial) => {
                        if total_read == read_buf.len() {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "HTTP headers exceeded buffer size",
                            ));
                        }
                        let n = client.read(&mut read_buf[total_read..]).await?;
                        if n == 0 {
                            return Ok(()); // Client disconnected
                        }
                        metrics.add_bytes_received(n as u64);
                        total_read += n;
                    }
                    Err(e) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("HTTP parse error: {:?}", e),
                        ));
                    }
                }
            };

            let req_total_len = header_len + body_len;

            // 2. Read remainder of body if not yet in buffer
            while total_read < req_total_len {
                if total_read == read_buf.len() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "HTTP body exceeded buffer size",
                    ));
                }
                let n = client.read(&mut read_buf[total_read..]).await?;
                if n == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Client disconnected before sending complete body",
                    ));
                }
                metrics.add_bytes_received(n as u64);
                total_read += n;
            }

            // 3. Inspect headers and construct forwarded request with zero allocations
            let mut headers = [httparse::EMPTY_HEADER; 64];
            let mut req = httparse::Request::new(&mut headers);
            let _ = req.parse(&read_buf[..header_len]);

            let method = req.method.unwrap_or("GET");
            let path = req.path.unwrap_or("/");

            let mut is_websocket = false;
            let mut client_close = false;
            let mut incoming_traceparent: Option<&str> = None;
            let mut host_hdr = "localhost";
            let mut if_none_match: Option<&str> = None;

            for h in req.headers.iter() {
                if h.name.eq_ignore_ascii_case("upgrade")
                    && h.value.eq_ignore_ascii_case(b"websocket")
                {
                    is_websocket = true;
                } else if h.name.eq_ignore_ascii_case("connection")
                    && h.value.eq_ignore_ascii_case(b"close")
                {
                    client_close = true;
                } else if h.name.eq_ignore_ascii_case("traceparent") {
                    incoming_traceparent = std::str::from_utf8(h.value).ok();
                } else if h.name.eq_ignore_ascii_case("host") {
                    if let Ok(s) = std::str::from_utf8(h.value) {
                        host_hdr = s;
                    }
                } else if h.name.eq_ignore_ascii_case("if-none-match") {
                    if_none_match = std::str::from_utf8(h.value).ok();
                }
            }

            // 4. RFC 7234 HTTP Cache lookup
            match cache.get(method, host_hdr, path, if_none_match) {
                nexuslb_cache::CacheResult::Hit(cached) => {
                    let mut resp_str = format!(
                        "HTTP/1.1 {} OK\r\nAge: {}\r\nX-Cache: HIT\r\nContent-Length: {}\r\n",
                        cached.status,
                        cached.age_secs(),
                        cached.body.len()
                    );
                    if let Some(ref tag) = cached.etag {
                        resp_str.push_str(&format!("ETag: {}\r\n", tag));
                    }
                    for (k, v) in &cached.headers {
                        if !k.eq_ignore_ascii_case("content-length")
                            && !k.eq_ignore_ascii_case("age")
                            && !k.eq_ignore_ascii_case("etag")
                        {
                            resp_str.push_str(&format!("{}: {}\r\n", k, v));
                        }
                    }
                    resp_str.push_str("\r\n");
                    client.write_all(resp_str.as_bytes()).await?;
                    client.write_all(&cached.body).await?;
                    metrics.inc_requests();

                    let leftover = total_read.saturating_sub(req_total_len);
                    if leftover > 0 {
                        read_buf.copy_within(req_total_len..total_read, 0);
                    }
                    total_read = leftover;

                    if client_close {
                        return Ok(());
                    }
                    continue;
                }
                nexuslb_cache::CacheResult::NotModified(etag) => {
                    let mut resp_str =
                        "HTTP/1.1 304 Not Modified\r\nX-Cache: HIT-REVALIDATED\r\n".to_string();
                    if let Some(ref tag) = etag {
                        resp_str.push_str(&format!("ETag: {}\r\n", tag));
                    }
                    resp_str.push_str("\r\n");
                    client.write_all(resp_str.as_bytes()).await?;
                    metrics.inc_requests();

                    let leftover = total_read.saturating_sub(req_total_len);
                    if leftover > 0 {
                        read_buf.copy_within(req_total_len..total_read, 0);
                    }
                    total_read = leftover;

                    if client_close {
                        return Ok(());
                    }
                    continue;
                }
                nexuslb_cache::CacheResult::Miss => {}
            }

            // 5. Build forwarded request in reusable req_bytes
            let span = nexuslb_observability::ProxySpan::new(incoming_traceparent);
            req_bytes.clear();
            let _ = write!(&mut req_bytes, "{} {} HTTP/1.1\r\n", method, path);

            for h in req.headers.iter() {
                if !h.name.eq_ignore_ascii_case("x-forwarded-for")
                    && !h.name.eq_ignore_ascii_case("x-forwarded-proto")
                    && !h.name.eq_ignore_ascii_case("traceparent")
                {
                    req_bytes.extend_from_slice(h.name.as_bytes());
                    req_bytes.extend_from_slice(b": ");
                    req_bytes.extend_from_slice(h.value);
                    req_bytes.extend_from_slice(b"\r\n");
                }
            }

            let proto_str = if is_tls { "https" } else { "http" };
            let _ = write!(&mut req_bytes, "X-Forwarded-For: {}\r\n", client_addr.ip());
            let _ = write!(&mut req_bytes, "X-Forwarded-Proto: {}\r\n", proto_str);
            let _ = write!(
                &mut req_bytes,
                "traceparent: {}\r\n\r\n",
                span.trace_context.to_header_value()
            );

            if body_len > 0 {
                req_bytes.extend_from_slice(&read_buf[header_len..req_total_len]);
            }

            // Shift leftover bytes in read_buf to front
            let leftover = total_read.saturating_sub(req_total_len);
            if leftover > 0 {
                read_buf.copy_within(req_total_len..total_read, 0);
            }
            total_read = leftover;

            let upstream = conn_pool
                .get_or_connect(backend.id(), backend.socket_addr())
                .await
                .map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::NotConnected, e.to_string())
                })?;

            if is_websocket {
                return Self::handle_websocket_upgrade(
                    client,
                    upstream,
                    &req_bytes,
                    backend,
                    metrics,
                    buffer_pool,
                )
                .await;
            } else {
                Self::forward_http_request(
                    &mut client,
                    upstream,
                    &req_bytes,
                    backend.clone(),
                    conn_pool.clone(),
                    metrics.clone(),
                    &mut resp_buf,
                )
                .await?;

                if client_close {
                    return Ok(());
                }
            }
        }
    }

    async fn handle_websocket_upgrade(
        client: TcpStream,
        mut upstream: TcpStream,
        initial_payload: &[u8],
        backend: Arc<Backend>,
        metrics: Arc<WorkerMetrics>,
        buffer_pool: BufferPool,
    ) -> std::io::Result<()> {
        trace!("Handling WebSocket handshake upgrade");
        upstream.write_all(initial_payload).await?;
        TcpProxy::forward(client, upstream, backend, metrics, buffer_pool).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn forward_http_request(
        client: &mut TcpStream,
        mut upstream: TcpStream,
        initial_payload: &[u8],
        backend: Arc<Backend>,
        conn_pool: ConnectionPool,
        metrics: Arc<WorkerMetrics>,
        resp_buf: &mut [u8],
    ) -> std::io::Result<()> {
        let start = Instant::now();
        metrics.inc_requests();
        backend.stats().inc_requests();
        backend.stats().inc_active_connections();

        // Write request to upstream
        upstream.write_all(initial_payload).await?;

        // Read response headers from upstream
        let mut total_read = 0;
        let mut header_len = 0;
        let mut content_length = None;
        let mut is_close = false;

        loop {
            let n = upstream.read(&mut resp_buf[total_read..]).await?;
            if n == 0 {
                break;
            }
            total_read += n;

            let mut headers = [httparse::EMPTY_HEADER; 64];
            let mut resp = httparse::Response::new(&mut headers);

            if let Ok(httparse::Status::Complete(hlen)) = resp.parse(&resp_buf[..total_read]) {
                header_len = hlen;
                for h in resp.headers.iter() {
                    if h.name.eq_ignore_ascii_case("content-length") {
                        if let Ok(s) = std::str::from_utf8(h.value) {
                            content_length = s.trim().parse::<usize>().ok();
                        }
                    }
                    if h.name.eq_ignore_ascii_case("connection")
                        && h.value.eq_ignore_ascii_case(b"close")
                    {
                        is_close = true;
                    }
                }
                break;
            }

            if total_read == resp_buf.len() {
                break;
            }
        }

        // Stream parsed bytes to client
        if total_read > 0 {
            client.write_all(&resp_buf[..total_read]).await?;
        }

        let mut total_bytes = total_read as u64;

        // If Content-Length is present, read remaining body bytes
        if let Some(cl) = content_length {
            let body_read = total_read.saturating_sub(header_len);
            let mut remaining = cl.saturating_sub(body_read);

            while remaining > 0 {
                let to_read = remaining.min(resp_buf.len());
                let n = upstream.read(&mut resp_buf[..to_read]).await?;
                if n == 0 {
                    break;
                }
                client.write_all(&resp_buf[..n]).await?;
                remaining -= n;
                total_bytes += n as u64;
            }
        } else if is_close {
            // No Content-Length specified: read until EOF
            loop {
                let n = upstream.read(resp_buf).await?;
                if n == 0 {
                    break;
                }
                client.write_all(&resp_buf[..n]).await?;
                total_bytes += n as u64;
            }
        }

        let duration = start.elapsed();
        backend.stats().dec_active_connections();
        backend
            .stats()
            .record_success(duration, initial_payload.len() as u64, total_bytes);
        metrics.record_latency(duration);
        metrics.add_bytes_sent(total_bytes);

        // For HTTP/1.1 keep-alive, return upstream connection if not closed
        if !is_close {
            conn_pool.return_connection(backend.id(), upstream);
        }

        Ok(())
    }
}
