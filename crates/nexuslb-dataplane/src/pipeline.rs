use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, trace, warn};

use crate::state::SharedDataplaneState;
use nexuslb_metrics::WorkerMetrics;
use nexuslb_proxy::{H2Proxy, HttpProxy, TcpProxy};
use nexuslb_scheduler::traits::SelectionContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedProtocol {
    Tls,
    Http,
    Http2,
    RawTcp,
}

pub struct DataplanePipeline;

impl DataplanePipeline {
    #[inline(always)]
    pub async fn process_connection(
        mut client: TcpStream,
        client_addr: SocketAddr,
        state: Arc<SharedDataplaneState>,
        metrics: Arc<WorkerMetrics>,
    ) {
        let current_state = state.load();

        // 1. Rate limiting check
        if !current_state.rate_limiter.check(Some(client_addr.ip())) {
            metrics.inc_dropped_connections();
            let _ = client.write_all(b"HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nRetry-After: 1\r\nConnection: close\r\nContent-Length: 57\r\n\r\n{\"error\":\"Too Many Requests\",\"message\":\"Rate limit exceeded\"}\n").await;
            return;
        }

        metrics.inc_connections();
        struct ConnectionGuard(Arc<WorkerMetrics>);
        impl Drop for ConnectionGuard {
            fn drop(&mut self) {
                self.0.dec_connections();
            }
        }
        let _guard = ConnectionGuard(metrics.clone());

        // 2. Read initial bytes directly into pooled buffer (zero MSG_PEEK overhead)
        let mut read_buf = current_state.buffer_pool.acquire();
        let read_n = match client.read(&mut read_buf[..]).await {
            Ok(n) if n > 0 => n,
            _ => return, // Client disconnected or error
        };

        metrics.add_bytes_received(read_n as u64);
        let protocol = Self::detect_protocol(&read_buf[..read_n]);

        match protocol {
            DetectedProtocol::Http => {
                Self::handle_http(
                    client,
                    client_addr,
                    &current_state,
                    metrics,
                    false,
                    read_buf,
                    read_n,
                )
                .await;
            }
            DetectedProtocol::Http2 => {
                let prefixed =
                    nexuslb_network::PrefixedStream::new(read_buf[..read_n].to_vec(), client);
                Self::handle_http2(prefixed, client_addr, &current_state, metrics).await;
            }
            DetectedProtocol::RawTcp => {
                let prefixed =
                    nexuslb_network::PrefixedStream::new(read_buf[..read_n].to_vec(), client);
                Self::handle_tcp(prefixed, client_addr, &current_state, metrics).await;
            }
            DetectedProtocol::Tls => {
                // If TLS acceptor configured, terminate TLS; otherwise passthrough
                if let Some(ref acceptor) = current_state.tls_acceptor {
                    let prefixed =
                        nexuslb_network::PrefixedStream::new(read_buf[..read_n].to_vec(), client);
                    match acceptor.accept(prefixed).await {
                        Ok(mut tls_stream) => {
                            trace!("Terminated TLS handshake successfully");
                            let mut inner_buf = current_state.buffer_pool.acquire();
                            let inner_n = match tls_stream.read(&mut inner_buf[..]).await {
                                Ok(n) if n > 0 => n,
                                _ => return,
                            };
                            let inner_proto = Self::detect_protocol(&inner_buf[..inner_n]);
                            match inner_proto {
                                DetectedProtocol::Http2 => {
                                    // HTTP/2 stream over TLS
                                    let pool = current_state
                                        .router
                                        .default_pool()
                                        .or_else(|| current_state.router.pools().values().next());
                                    if let Some(pool) = pool {
                                        let ctx = SelectionContext::with_ip(client_addr.ip());
                                        if let Some(b) = pool.select(&ctx) {
                                            let prefixed_tls = nexuslb_network::PrefixedStream::new(
                                                inner_buf[..inner_n].to_vec(),
                                                tls_stream,
                                            );
                                            let _ = H2Proxy::handle_connection(
                                                prefixed_tls,
                                                client_addr,
                                                b,
                                                metrics,
                                            )
                                            .await;
                                        }
                                    }
                                }
                                _ => {
                                    // Default HTTP/1.1 over TLS
                                    Self::handle_http(
                                        tls_stream,
                                        client_addr,
                                        &current_state,
                                        metrics,
                                        true,
                                        inner_buf,
                                        inner_n,
                                    )
                                    .await;
                                }
                            }
                        }
                        Err(e) => {
                            debug!(error = %e, "TLS handshake failed");
                        }
                    }
                } else {
                    // TLS passthrough over TCP: preserve ClientHello bytes
                    let prefixed =
                        nexuslb_network::PrefixedStream::new(read_buf[..read_n].to_vec(), client);
                    Self::handle_tcp(prefixed, client_addr, &current_state, metrics).await;
                }
            }
        }
    }

    #[inline(always)]
    pub fn detect_protocol(peek: &[u8]) -> DetectedProtocol {
        if peek.is_empty() {
            return DetectedProtocol::RawTcp;
        }

        // Direct first-byte dispatch: eliminates loop over 8 prefixes
        match peek[0] {
            0x16 => {
                // TLS ClientHello: 0x16 0x03 0x0X
                if peek.len() >= 3 && peek[1] == 0x03 {
                    return DetectedProtocol::Tls;
                }
                DetectedProtocol::RawTcp
            }
            b'P' => {
                if peek.starts_with(b"PRI * HTTP/2.0") {
                    DetectedProtocol::Http2
                } else if peek.len() >= 4 && (peek[1] == b'O' || peek[1] == b'U' || peek[1] == b'A')
                {
                    // POST, PUT, PATCH
                    DetectedProtocol::Http
                } else {
                    DetectedProtocol::RawTcp
                }
            }
            b'G' => DetectedProtocol::Http, // GET
            b'H' => DetectedProtocol::Http, // HEAD
            b'D' => DetectedProtocol::Http, // DELETE
            b'O' => DetectedProtocol::Http, // OPTIONS
            b'C' => DetectedProtocol::Http, // CONNECT
            _ => DetectedProtocol::RawTcp,
        }
    }

    /// Handle HTTP/1.1 or HTTP/2 client stream with strict authoritative parsing.
    #[allow(clippy::too_many_arguments)]
    async fn handle_http<S>(
        mut client: S,
        client_addr: SocketAddr,
        state: &crate::state::DataplaneState,
        metrics: Arc<WorkerMetrics>,
        is_tls: bool,
        read_buf: nexuslb_network::PooledBuffer,
        initial_read: usize,
    ) where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        // Parse request line / Host header from initial read to evaluate routing rules
        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut req = httparse::Request::new(&mut headers);
        let (method, path, host) = if let Ok(httparse::Status::Complete(_))
        | Ok(httparse::Status::Partial) =
            req.parse(&read_buf[..initial_read])
        {
            let m = req.method;
            let p = req.path.unwrap_or("/");
            let h = req
                .headers
                .iter()
                .find(|hdr| hdr.name.eq_ignore_ascii_case("host"))
                .and_then(|hdr| std::str::from_utf8(hdr.value).ok());
            (m, p, h)
        } else {
            (None, "/", None)
        };

        // Match against routing table: first check configured routes, then default pool
        let pool = state
            .router
            .route(host, path, method, None, None)
            .map(|(_route, pool)| pool)
            .or_else(|| state.router.default_pool())
            .or_else(|| state.router.pools().values().next());

        let pool = match pool {
            Some(p) => p,
            None => {
                warn!("No backend pool configured in router");
                return;
            }
        };

        let ctx = SelectionContext::with_ip(client_addr.ip());
        let backend = match pool.select(&ctx) {
            Some(b) => b,
            None => {
                metrics.inc_backend_errors();
                warn!(pool = %pool.name, "No healthy backend available in pool");
                let _ = client
                    .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: 63\r\n\r\n{\"error\":\"Service Unavailable\",\"message\":\"No backends available\"}\n")
                    .await;
                return;
            }
        };

        metrics.inc_backend_requests();
        let _ = HttpProxy::handle_connection(
            &mut client,
            client_addr,
            backend,
            state.conn_pool.clone(),
            metrics,
            state.buffer_pool.clone(),
            state.retry_policy.clone(),
            is_tls,
            state.http_cache.clone(),
            read_buf,
            initial_read,
            state.access_logger.clone(),
            state.filter_chain.clone(),
            state.redirect_http_to_https,
        )
        .await;

        let _ = client.shutdown().await;
    }

    async fn handle_http2<S>(
        client: S,
        client_addr: SocketAddr,
        state: &crate::state::DataplaneState,
        metrics: Arc<WorkerMetrics>,
    ) where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let pool = state
            .router
            .default_pool()
            .or_else(|| state.router.pools().values().next());

        let pool = match pool {
            Some(p) => p,
            None => {
                warn!("No backend pool configured in router for HTTP/2");
                return;
            }
        };

        let ctx = SelectionContext::with_ip(client_addr.ip());
        let backend = match pool.select(&ctx) {
            Some(b) => b,
            None => {
                metrics.inc_backend_errors();
                return;
            }
        };

        metrics.inc_backend_requests();
        let _ =
            nexuslb_proxy::H2Proxy::handle_connection(client, client_addr, backend, metrics).await;
    }

    async fn handle_tcp<S>(
        client: S,
        client_addr: SocketAddr,
        state: &crate::state::DataplaneState,
        metrics: Arc<WorkerMetrics>,
    ) where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let pool = state
            .router
            .default_pool()
            .or_else(|| state.router.pools().values().next());

        let pool = match pool {
            Some(p) => p,
            None => {
                warn!("No backend pool configured in router");
                return;
            }
        };

        let ctx = SelectionContext::with_ip(client_addr.ip());
        let backend = match pool.select(&ctx) {
            Some(b) => b,
            None => {
                metrics.inc_backend_errors();
                warn!(pool = %pool.name, "No healthy backend available in pool");
                return;
            }
        };

        metrics.inc_backend_requests();
        match state
            .conn_pool
            .get_or_connect(backend.id(), backend.socket_addr())
            .await
        {
            Ok(upstream) => {
                let _ = TcpProxy::forward(
                    client,
                    upstream,
                    backend,
                    metrics,
                    state.buffer_pool.clone(),
                )
                .await;
            }
            Err(e) => {
                metrics.inc_backend_errors();
                debug!(backend = %backend.name(), error = %e, "Failed to connect to upstream");
            }
        }
    }
}
