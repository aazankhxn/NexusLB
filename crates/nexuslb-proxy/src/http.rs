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
    /// Proxy an HTTP/1.1 or WebSocket request from client to selected backend
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
    ) -> std::io::Result<()> {
        let mut read_buf = buffer_pool.acquire();
        let mut total_read = 0;

        // Read initial request bytes
        loop {
            let n = client.read(&mut read_buf[total_read..]).await?;
            if n == 0 {
                return Ok(()); // Client disconnected
            }
            total_read += n;

            let mut headers = [httparse::EMPTY_HEADER; 64];
            let mut req = httparse::Request::new(&mut headers);

            match req.parse(&read_buf[..total_read]) {
                Ok(httparse::Status::Complete(header_len)) => {
                    let method = req.method.unwrap_or("GET");
                    let path = req.path.unwrap_or("/");

                    // Check for WebSocket upgrade
                    let mut is_websocket = false;
                    let mut content_length: Option<usize> = None;

                    for h in req.headers.iter() {
                        if h.name.eq_ignore_ascii_case("upgrade")
                            && h.value.eq_ignore_ascii_case(b"websocket")
                        {
                            is_websocket = true;
                        }
                        if h.name.eq_ignore_ascii_case("content-length") {
                            if let Ok(s) = std::str::from_utf8(h.value) {
                                content_length = s.trim().parse::<usize>().ok();
                            }
                        }
                    }

                    // Build forwarded request
                    let mut req_bytes = Vec::with_capacity(header_len + 256);
                    req_bytes
                        .extend_from_slice(format!("{} {} HTTP/1.1\r\n", method, path).as_bytes());

                    // Copy headers, adding X-Forwarded headers
                    for h in req.headers.iter() {
                        if !h.name.eq_ignore_ascii_case("x-forwarded-for")
                            && !h.name.eq_ignore_ascii_case("x-forwarded-proto")
                        {
                            req_bytes.extend_from_slice(h.name.as_bytes());
                            req_bytes.extend_from_slice(b": ");
                            req_bytes.extend_from_slice(h.value);
                            req_bytes.extend_from_slice(b"\r\n");
                        }
                    }

                    let proto_str = if is_tls { "https" } else { "http" };
                    req_bytes.extend_from_slice(
                        format!("X-Forwarded-For: {}\r\n", client_addr.ip()).as_bytes(),
                    );
                    req_bytes.extend_from_slice(
                        format!("X-Forwarded-Proto: {}\r\n\r\n", proto_str).as_bytes(),
                    );

                    // If there was any body data read alongside headers, append it
                    if total_read > header_len {
                        req_bytes.extend_from_slice(&read_buf[header_len..total_read]);
                    }

                    // Connect or acquire connection from pool
                    let upstream = conn_pool
                        .get_or_connect(backend.id(), backend.socket_addr())
                        .await
                        .map_err(|e| {
                            std::io::Error::new(std::io::ErrorKind::NotConnected, e.to_string())
                        })?;

                    if is_websocket {
                        // For WebSocket upgrade, switch immediately to bidirectional duplex proxy
                        return Self::handle_websocket_upgrade(
                            client,
                            upstream,
                            req_bytes,
                            backend,
                            metrics,
                            buffer_pool,
                        )
                        .await;
                    } else {
                        // Standard HTTP/1.1 forwarding
                        return Self::forward_http_request(
                            client,
                            upstream,
                            req_bytes,
                            content_length,
                            backend,
                            conn_pool,
                            metrics,
                            buffer_pool,
                        )
                        .await;
                    }
                }
                Ok(httparse::Status::Partial) => {
                    if total_read == read_buf.len() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "HTTP headers exceeded buffer size",
                        ));
                    }
                    continue;
                }
                Err(e) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("HTTP parse error: {:?}", e),
                    ));
                }
            }
        }
    }

    async fn handle_websocket_upgrade(
        client: TcpStream,
        mut upstream: TcpStream,
        initial_payload: Vec<u8>,
        backend: Arc<Backend>,
        metrics: Arc<WorkerMetrics>,
        buffer_pool: BufferPool,
    ) -> std::io::Result<()> {
        trace!("Handling WebSocket handshake upgrade");
        upstream.write_all(&initial_payload).await?;
        TcpProxy::forward(client, upstream, backend, metrics, buffer_pool).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn forward_http_request(
        mut client: TcpStream,
        mut upstream: TcpStream,
        initial_payload: Vec<u8>,
        _content_length: Option<usize>,
        backend: Arc<Backend>,
        conn_pool: ConnectionPool,
        metrics: Arc<WorkerMetrics>,
        buffer_pool: BufferPool,
    ) -> std::io::Result<()> {
        let start = Instant::now();
        metrics.inc_requests();
        backend.stats().inc_requests();
        backend.stats().inc_active_connections();

        // Write request to upstream
        upstream.write_all(&initial_payload).await?;

        // Read response from upstream and stream back to client
        let mut resp_buf = buffer_pool.acquire();
        let mut total_bytes = 0u64;

        loop {
            let n = upstream.read(&mut resp_buf).await?;
            if n == 0 {
                break;
            }
            client.write_all(&resp_buf[..n]).await?;
            total_bytes += n as u64;
        }

        let duration = start.elapsed();
        backend.stats().dec_active_connections();
        backend
            .stats()
            .record_success(duration, initial_payload.len() as u64, total_bytes);
        metrics.record_latency(duration);
        metrics.add_bytes_sent(total_bytes);

        // For HTTP/1.1 keep-alive, return upstream connection if still alive
        conn_pool.return_connection(backend.id(), upstream);

        Ok(())
    }
}
