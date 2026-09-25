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
    ) -> std::io::Result<()> {
        let mut read_buf = buffer_pool.acquire();
        let mut total_read = 0;

        loop {
            // Read and parse an HTTP request header
            let (header_len, method, path, is_websocket, client_close, content_length) = loop {
                let mut headers = [httparse::EMPTY_HEADER; 64];
                let mut req = httparse::Request::new(&mut headers);

                match req.parse(&read_buf[..total_read]) {
                    Ok(httparse::Status::Complete(hlen)) => {
                        let method = req.method.unwrap_or("GET").to_string();
                        let path = req.path.unwrap_or("/").to_string();

                        let mut is_ws = false;
                        let mut cl = None;
                        let mut close = false;

                        for h in req.headers.iter() {
                            if h.name.eq_ignore_ascii_case("upgrade")
                                && h.value.eq_ignore_ascii_case(b"websocket")
                            {
                                is_ws = true;
                            }
                            if h.name.eq_ignore_ascii_case("connection")
                                && h.value.eq_ignore_ascii_case(b"close")
                            {
                                close = true;
                            }
                            if h.name.eq_ignore_ascii_case("content-length") {
                                if let Ok(s) = std::str::from_utf8(h.value) {
                                    cl = s.trim().parse::<usize>().ok();
                                }
                            }
                        }
                        break (hlen, method, path, is_ws, close, cl);
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

            let body_len = content_length.unwrap_or(0);
            let req_total_len = header_len + body_len;

            // Read remainder of body if not yet in buffer
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
                total_read += n;
            }

            // Build forwarded request
            let mut req_bytes = Vec::with_capacity(header_len + 256 + body_len);
            req_bytes.extend_from_slice(format!("{} {} HTTP/1.1\r\n", method, path).as_bytes());

            // Re-parse headers to accurately reconstruct
            let mut headers = [httparse::EMPTY_HEADER; 64];
            let mut req = httparse::Request::new(&mut headers);
            let _ = req.parse(&read_buf[..header_len]);

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
            req_bytes
                .extend_from_slice(format!("X-Forwarded-For: {}\r\n", client_addr.ip()).as_bytes());
            req_bytes
                .extend_from_slice(format!("X-Forwarded-Proto: {}\r\n\r\n", proto_str).as_bytes());

            if body_len > 0 {
                req_bytes.extend_from_slice(&read_buf[header_len..req_total_len]);
            }

            // Move leftover bytes in read_buf to front
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
                    req_bytes,
                    backend,
                    metrics,
                    buffer_pool,
                )
                .await;
            } else {
                Self::forward_http_request(
                    &mut client,
                    upstream,
                    req_bytes,
                    content_length,
                    backend.clone(),
                    conn_pool.clone(),
                    metrics.clone(),
                    buffer_pool.clone(),
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
        client: &mut TcpStream,
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

        // Read response headers from upstream
        let mut resp_buf = buffer_pool.acquire();
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
                let n = upstream.read(&mut resp_buf).await?;
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
