use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{debug, trace, warn};

use crate::state::SharedDataplaneState;
use nexuslb_metrics::WorkerMetrics;
use nexuslb_proxy::{HttpProxy, TcpProxy};
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
            let _ = client.write_all(b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 1\r\nContent-Length: 21\r\n\r\nRate limit exceeded\n").await;
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

        // 2. Peek initial bytes for protocol detection (up to 24 bytes for HTTP/2 preface)
        let mut peek_buf = [0u8; 24];
        let peek_n = match client.peek(&mut peek_buf).await {
            Ok(n) if n > 0 => n,
            _ => return, // Client disconnected or error
        };

        let protocol = Self::detect_protocol(&peek_buf[..peek_n]);

        match protocol {
            DetectedProtocol::Http => {
                Self::handle_http(client, client_addr, &current_state, metrics, false).await;
            }
            DetectedProtocol::Http2 => {
                Self::handle_http2(client, client_addr, &current_state, metrics).await;
            }
            DetectedProtocol::RawTcp => {
                Self::handle_tcp(client, client_addr, &current_state, metrics).await;
            }
            DetectedProtocol::Tls => {
                // If TLS acceptor configured, we can terminate TLS or forward
                if let Some(ref acceptor) = current_state.tls_acceptor {
                    match acceptor.accept(client).await {
                        Ok(_tls_stream) => {
                            // Forward over TLS
                            trace!("Terminated TLS handshake successfully");
                        }
                        Err(e) => {
                            debug!(error = %e, "TLS handshake failed");
                        }
                    }
                } else {
                    // TLS passthrough over TCP
                    Self::handle_tcp(client, client_addr, &current_state, metrics).await;
                }
            }
        }
    }

    #[inline(always)]
    pub fn detect_protocol(peek: &[u8]) -> DetectedProtocol {
        if peek.len() >= 3 && peek[0] == 0x16 && peek[1] == 0x03 {
            return DetectedProtocol::Tls;
        }

        if peek.starts_with(b"PRI * HTTP/2.0") {
            return DetectedProtocol::Http2;
        }

        let http_prefixes = [
            b"GET ", b"POST", b"HEAD", b"PUT ", b"DELE", b"OPTI", b"PATC", b"CONN",
        ];

        for prefix in &http_prefixes {
            if peek.starts_with(*prefix) {
                return DetectedProtocol::Http;
            }
        }

        DetectedProtocol::RawTcp
    }

    async fn handle_http(
        client: TcpStream,
        client_addr: SocketAddr,
        state: &crate::state::DataplaneState,
        metrics: Arc<WorkerMetrics>,
        is_tls: bool,
    ) {
        // Quick route match: check default pool or route by path/host
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
                let mut client = client;
                let _ = client
                    .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 23\r\n\r\nNo backends available\n")
                    .await;
                return;
            }
        };

        metrics.inc_backend_requests();
        let _ = HttpProxy::handle_connection(
            client,
            client_addr,
            backend,
            state.conn_pool.clone(),
            metrics,
            state.buffer_pool.clone(),
            state.retry_policy.clone(),
            is_tls,
            state.http_cache.clone(),
        )
        .await;
    }

    async fn handle_http2(
        client: TcpStream,
        client_addr: SocketAddr,
        state: &crate::state::DataplaneState,
        metrics: Arc<WorkerMetrics>,
    ) {
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

    async fn handle_tcp(
        client: TcpStream,
        client_addr: SocketAddr,
        state: &crate::state::DataplaneState,
        metrics: Arc<WorkerMetrics>,
    ) {
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
