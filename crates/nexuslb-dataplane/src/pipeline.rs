use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, trace, warn};

use crate::state::SharedDataplaneState;
use nexuslb_core::backend::BackendConnectionGuard;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolState {
    Decided(DetectedProtocol),
    NeedMoreData,
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

        // 1b. Global server connection capacity check (prevent socket descriptor exhaustion)
        const MAX_DATAPLANE_CONNECTIONS: i64 = 100_000;
        if metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed) >= MAX_DATAPLANE_CONNECTIONS {
            metrics.inc_dropped_connections();
            let _ = client.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain\r\nConnection: close\r\nContent-Length: 32\r\n\r\nServer connection limit reached\n").await;
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

        // 2. Read initial bytes directly into pooled buffer with streaming protocol detection (Issue #3)
        // Guarded by probe timeout to prevent Slowloris connection pool exhaustion
        const CLIENT_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
        let mut read_buf = current_state.buffer_pool.acquire();
        let mut read_n = 0;
        let probe_result = tokio::time::timeout(CLIENT_PROBE_TIMEOUT, async {
            loop {
                let n = match client.read(&mut read_buf[read_n..]).await {
                    Ok(n) if n > 0 => n,
                    _ => return None, // Client disconnected or error
                };
                read_n += n;
                metrics.add_bytes_received(n as u64);

                match Self::inspect_protocol(&read_buf[..read_n]) {
                    ProtocolState::Decided(proto) => return Some(proto),
                    ProtocolState::NeedMoreData => {
                        // Maximum 24 bytes needed to distinguish all protocols (e.g. PRI * HTTP/2.0)
                        if read_n >= 24 {
                            return Some(DetectedProtocol::RawTcp);
                        }
                        continue;
                    }
                }
            }
        })
        .await;

        let protocol = match probe_result {
            Ok(Some(proto)) => proto,
            _ => return, // Timed out or error
        };

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
                    match tokio::time::timeout(CLIENT_PROBE_TIMEOUT, acceptor.accept(prefixed)).await {
                        Ok(Ok(mut tls_stream)) => {
                            trace!("Terminated TLS handshake successfully");
                            let mut inner_buf = current_state.buffer_pool.acquire();
                            let mut inner_n = 0;
                            let inner_probe = tokio::time::timeout(CLIENT_PROBE_TIMEOUT, async {
                                loop {
                                    let n = match tls_stream.read(&mut inner_buf[inner_n..]).await {
                                        Ok(n) if n > 0 => n,
                                        _ => return None,
                                    };
                                    inner_n += n;
                                    match Self::inspect_protocol(&inner_buf[..inner_n]) {
                                        ProtocolState::Decided(proto) => return Some(proto),
                                        ProtocolState::NeedMoreData => {
                                            if inner_n >= 24 {
                                                return Some(DetectedProtocol::Http);
                                            }
                                            continue;
                                        }
                                    }
                                }
                            })
                            .await;

                            let inner_proto = match inner_probe {
                                Ok(Some(proto)) => proto,
                                _ => return,
                            };

                            match inner_proto {
                                DetectedProtocol::Http2 => {
                                    // HTTP/2 stream over TLS: stream-level routing & multiplexing (Issues #1, #2, #10)
                                    let prefixed_tls = nexuslb_network::PrefixedStream::new(
                                        inner_buf[..inner_n].to_vec(),
                                        tls_stream,
                                    );
                                    let _ = H2Proxy::handle_connection(
                                        prefixed_tls,
                                        client_addr,
                                        current_state.router.clone(),
                                        current_state.h2_pool.clone(),
                                        metrics,
                                        current_state.h2_config.clone(),
                                        current_state.access_logger.clone(),
                                        current_state.filter_chain.clone(),
                                        current_state.rate_limiter.clone(),
                                        true,
                                    )
                                    .await;
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
                        Ok(Err(e)) => {
                            debug!(error = %e, "TLS handshake failed");
                        }
                        Err(_) => {
                            debug!("TLS handshake timed out");
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

    /// Streaming state machine to handle fragmented inputs under TCP semantics (Issue #3)
    #[inline(always)]
    pub fn inspect_protocol(peek: &[u8]) -> ProtocolState {
        if peek.is_empty() {
            return ProtocolState::NeedMoreData;
        }

        match peek[0] {
            0x16 => {
                // TLS ClientHello record: 0x16 0x03 [0x00..=0x04]
                if peek.len() < 3 {
                    return ProtocolState::NeedMoreData;
                }
                if peek[1] == 0x03 && peek[2] <= 0x04 {
                    ProtocolState::Decided(DetectedProtocol::Tls)
                } else {
                    ProtocolState::Decided(DetectedProtocol::RawTcp)
                }
            }
            b'P' => {
                // Could be HTTP/2 prior-knowledge ("PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
                // or HTTP/1 methods: POST, PUT, PATCH
                if peek.len() < 4 {
                    if b"PRI * HTTP/2.0".starts_with(peek)
                        || b"POST ".starts_with(peek)
                        || b"PUT ".starts_with(peek)
                        || b"PATCH ".starts_with(peek)
                    {
                        return ProtocolState::NeedMoreData;
                    }
                    return ProtocolState::Decided(DetectedProtocol::RawTcp);
                }

                if peek.starts_with(b"PRI ") {
                    if peek.len() < 14 {
                        return ProtocolState::NeedMoreData;
                    }
                    if peek.starts_with(b"PRI * HTTP/2.0") {
                        ProtocolState::Decided(DetectedProtocol::Http2)
                    } else {
                        ProtocolState::Decided(DetectedProtocol::RawTcp)
                    }
                } else if peek.starts_with(b"POST ")
                    || peek.starts_with(b"PUT ")
                    || peek.starts_with(b"PATCH ")
                {
                    ProtocolState::Decided(DetectedProtocol::Http)
                } else if peek.len() < 6
                    && (b"PATCH ".starts_with(peek) || b"POST ".starts_with(peek))
                {
                    ProtocolState::NeedMoreData
                } else {
                    ProtocolState::Decided(DetectedProtocol::RawTcp)
                }
            }
            b'G' => {
                if peek.len() < 4 {
                    if b"GET ".starts_with(peek) {
                        return ProtocolState::NeedMoreData;
                    }
                    return ProtocolState::Decided(DetectedProtocol::RawTcp);
                }
                if peek.starts_with(b"GET ") {
                    ProtocolState::Decided(DetectedProtocol::Http)
                } else {
                    ProtocolState::Decided(DetectedProtocol::RawTcp)
                }
            }
            b'H' => {
                if peek.len() < 5 {
                    if b"HEAD ".starts_with(peek) {
                        return ProtocolState::NeedMoreData;
                    }
                    return ProtocolState::Decided(DetectedProtocol::RawTcp);
                }
                if peek.starts_with(b"HEAD ") {
                    ProtocolState::Decided(DetectedProtocol::Http)
                } else {
                    ProtocolState::Decided(DetectedProtocol::RawTcp)
                }
            }
            b'D' => {
                if peek.len() < 7 {
                    if b"DELETE ".starts_with(peek) {
                        return ProtocolState::NeedMoreData;
                    }
                    return ProtocolState::Decided(DetectedProtocol::RawTcp);
                }
                if peek.starts_with(b"DELETE ") {
                    ProtocolState::Decided(DetectedProtocol::Http)
                } else {
                    ProtocolState::Decided(DetectedProtocol::RawTcp)
                }
            }
            b'O' => {
                if peek.len() < 8 {
                    if b"OPTIONS ".starts_with(peek) {
                        return ProtocolState::NeedMoreData;
                    }
                    return ProtocolState::Decided(DetectedProtocol::RawTcp);
                }
                if peek.starts_with(b"OPTIONS ") {
                    ProtocolState::Decided(DetectedProtocol::Http)
                } else {
                    ProtocolState::Decided(DetectedProtocol::RawTcp)
                }
            }
            b'C' => {
                if peek.len() < 8 {
                    if b"CONNECT ".starts_with(peek) {
                        return ProtocolState::NeedMoreData;
                    }
                    return ProtocolState::Decided(DetectedProtocol::RawTcp);
                }
                if peek.starts_with(b"CONNECT ") {
                    ProtocolState::Decided(DetectedProtocol::Http)
                } else {
                    ProtocolState::Decided(DetectedProtocol::RawTcp)
                }
            }
            _ => ProtocolState::Decided(DetectedProtocol::RawTcp),
        }
    }

    #[inline(always)]
    pub fn detect_protocol(peek: &[u8]) -> DetectedProtocol {
        match Self::inspect_protocol(peek) {
            ProtocolState::Decided(proto) => proto,
            ProtocolState::NeedMoreData => DetectedProtocol::RawTcp,
        }
    }

    /// Handle HTTP/1.1 or HTTP/2 client stream with strict authoritative parsing.
    #[allow(clippy::too_many_arguments)]
    async fn handle_http<S>(
        client: S,
        client_addr: SocketAddr,
        state: &crate::state::DataplaneState,
        metrics: Arc<WorkerMetrics>,
        is_tls: bool,
        read_buf: nexuslb_network::PooledBuffer,
        initial_read: usize,
    ) where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let _ = HttpProxy::handle_connection(
            client,
            client_addr,
            state.router.clone(),
            state.rate_limiter.clone(),
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
    }

    async fn handle_http2<S>(
        client: S,
        client_addr: SocketAddr,
        state: &crate::state::DataplaneState,
        metrics: Arc<WorkerMetrics>,
    ) where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let _ = nexuslb_proxy::H2Proxy::handle_connection(
            client,
            client_addr,
            state.router.clone(),
            state.h2_pool.clone(),
            metrics,
            state.h2_config.clone(),
            state.access_logger.clone(),
            state.filter_chain.clone(),
            state.rate_limiter.clone(),
            false,
        )
        .await;
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

        let _guard = match BackendConnectionGuard::try_acquire(backend.clone()) {
            Some(g) => g,
            None => {
                metrics.inc_backend_errors();
                warn!(backend = %backend.name(), "Backend connection limit reached (max_connections)");
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
                    backend.clone(),
                    metrics,
                    state.buffer_pool.clone(),
                )
                .await;
                state.conn_pool.dec_active(backend.id());
            }
            Err(e) => {
                metrics.inc_backend_errors();
                debug!(backend = %backend.name(), error = %e, "Failed to connect to upstream");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fragmented_protocol_detection_issue_3() {
        // 1. Fragmented TLS ClientHello (Issue #3)
        // 1 byte: 0x16 alone MUST NOT be assumed RawTcp
        assert_eq!(
            DataplanePipeline::inspect_protocol(&[0x16]),
            ProtocolState::NeedMoreData
        );
        // 2 bytes: 0x16 0x03 still needs version minor byte
        assert_eq!(
            DataplanePipeline::inspect_protocol(&[0x16, 0x03]),
            ProtocolState::NeedMoreData
        );
        // 3 bytes: 0x16 0x03 0x01 (TLS 1.0) -> Decided TLS
        assert_eq!(
            DataplanePipeline::inspect_protocol(&[0x16, 0x03, 0x01]),
            ProtocolState::Decided(DetectedProtocol::Tls)
        );
        // 3 bytes: 0x16 0x03 0x03 (TLS 1.2 / 1.3) -> Decided TLS
        assert_eq!(
            DataplanePipeline::inspect_protocol(&[0x16, 0x03, 0x03]),
            ProtocolState::Decided(DetectedProtocol::Tls)
        );

        // 2. Fragmented HTTP/2 prior knowledge preface
        assert_eq!(
            DataplanePipeline::inspect_protocol(b"P"),
            ProtocolState::NeedMoreData
        );
        assert_eq!(
            DataplanePipeline::inspect_protocol(b"PRI * "),
            ProtocolState::NeedMoreData
        );
        assert_eq!(
            DataplanePipeline::inspect_protocol(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n"),
            ProtocolState::Decided(DetectedProtocol::Http2)
        );

        // 3. Fragmented HTTP/1.1 methods
        assert_eq!(
            DataplanePipeline::inspect_protocol(b"G"),
            ProtocolState::NeedMoreData
        );
        assert_eq!(
            DataplanePipeline::inspect_protocol(b"GET "),
            ProtocolState::Decided(DetectedProtocol::Http)
        );
        assert_eq!(
            DataplanePipeline::inspect_protocol(b"POST /api HTTP/1.1"),
            ProtocolState::Decided(DetectedProtocol::Http)
        );

        // 4. Raw TCP non-matching protocol (e.g. SSH, MySQL)
        assert_eq!(
            DataplanePipeline::inspect_protocol(b"SSH-2.0-OpenSSH"),
            ProtocolState::Decided(DetectedProtocol::RawTcp)
        );
    }
}
