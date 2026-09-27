use bytes::Bytes;
use h2::server;
use http::{Request, Response};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tracing::{debug, error, trace};

use nexuslb_core::backend::Backend;
use nexuslb_metrics::WorkerMetrics;

struct ConnGuard(Arc<Backend>);
impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.0.stats().dec_active_connections();
    }
}

/// High-performance HTTP/2 and gRPC multiplexed stream proxy
pub struct H2Proxy;

impl H2Proxy {
    /// Accept HTTP/2 client connection and multiplex individual streams to backends
    pub async fn handle_connection<S>(
        client: S,
        client_addr: SocketAddr,
        backend: Arc<Backend>,
        metrics: Arc<WorkerMetrics>,
    ) -> std::io::Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let mut connection = server::Builder::new()
            .max_concurrent_streams(128)
            .handshake(client)
            .await
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset,
                    format!("H2 handshake error: {:?}", e),
                )
            })?;

        trace!(client = %client_addr, "HTTP/2 connection handshake established");

        let stream_semaphore = Arc::new(tokio::sync::Semaphore::new(128));

        while let Some(result) = connection.accept().await {
            let (req, mut respond) = match result {
                Ok(stream) => stream,
                Err(e) => {
                    debug!("H2 accept stream error: {:?}", e);
                    break;
                }
            };

            let permit = match stream_semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    debug!("H2 connection exceeded max concurrent active streams (128)");
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

            let backend = backend.clone();
            let metrics = metrics.clone();

            tokio::spawn(async move {
                let _permit = permit;
                let start = Instant::now();
                metrics.inc_requests();
                backend.stats().inc_requests();
                backend.stats().inc_active_connections();
                let _guard = ConnGuard(backend.clone());

                if let Err(e) = Self::proxy_stream(req, &mut respond, backend.clone()).await {
                    error!(error = %e, "H2 stream proxy error");
                    backend.stats().record_error();
                    let response = Response::builder()
                        .status(502)
                        .header("content-type", "text/plain")
                        .body(())
                        .unwrap();
                    let mut send_stream = respond.send_response(response, false).ok();
                    if let Some(ref mut s) = send_stream {
                        let _ = s.send_data(Bytes::from_static(b"Bad Gateway\n"), true);
                    }
                } else {
                    let elapsed = start.elapsed();
                    backend.stats().record_success(elapsed, 0, 0);
                    metrics.record_latency(elapsed);
                }
            });
        }

        Ok(())
    }

    async fn proxy_stream(
        client_req: Request<h2::RecvStream>,
        client_respond: &mut server::SendResponse<Bytes>,
        backend: Arc<Backend>,
    ) -> anyhow::Result<()> {
        const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
        const STREAM_CHUNK_TIMEOUT: Duration = Duration::from_secs(30);

        // Connect to upstream backend with 5s timeout
        let upstream_stream = tokio::time::timeout(
            CONNECT_TIMEOUT,
            TcpStream::connect(backend.socket_addr()),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Upstream H2 connect timed out after 5s"))??;
        let _ = upstream_stream.set_nodelay(true);

        let (mut client_h2, upstream_conn) = tokio::time::timeout(
            CONNECT_TIMEOUT,
            h2::client::handshake(upstream_stream),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Upstream H2 handshake timed out after 5s"))??;

        // Drive upstream connection in background — track handle to abort on completion
        let driver_handle = tokio::spawn(async move {
            if let Err(e) = upstream_conn.await {
                debug!("Upstream H2 connection closed: {:?}", e);
            }
        });

        // Ensure driver task is aborted when proxy_stream returns (success or error)
        struct DriverGuard(tokio::task::JoinHandle<()>);
        impl Drop for DriverGuard {
            fn drop(&mut self) {
                self.0.abort();
            }
        }
        let _driver_guard = DriverGuard(driver_handle);

        // Forward request headers
        let (parts, mut body_recv) = client_req.into_parts();
        let upstream_req = Request::from_parts(parts, ());
        let is_end_of_stream = body_recv.is_end_stream();

        let (resp_fut, mut upstream_send_body) =
            client_h2.send_request(upstream_req, is_end_of_stream)?;

        // Forward client streaming data frames to upstream with chunk timeout
        if !is_end_of_stream {
            loop {
                let chunk = tokio::time::timeout(STREAM_CHUNK_TIMEOUT, body_recv.data())
                    .await
                    .map_err(|_| anyhow::anyhow!("Client H2 stream chunk timed out after 30s"))?;
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
            let trailers = tokio::time::timeout(STREAM_CHUNK_TIMEOUT, body_recv.trailers())
                .await
                .map_err(|_| anyhow::anyhow!("Client H2 stream trailers timed out"))??;
            if let Some(trailers) = trailers {
                upstream_send_body.send_trailers(trailers)?;
            }
        }

        // Await upstream response headers with 30s timeout
        let upstream_resp = tokio::time::timeout(STREAM_CHUNK_TIMEOUT, resp_fut)
            .await
            .map_err(|_| anyhow::anyhow!("Upstream H2 response timed out after 30s"))??;
        let (resp_parts, mut upstream_resp_body) = upstream_resp.into_parts();
        let client_resp = Response::from_parts(resp_parts, ());
        let resp_is_eos = upstream_resp_body.is_end_stream();

        let mut client_send_body = client_respond.send_response(client_resp, resp_is_eos)?;

        // Forward upstream streaming response data frames to client with chunk timeout
        if !resp_is_eos {
            loop {
                let chunk = tokio::time::timeout(STREAM_CHUNK_TIMEOUT, upstream_resp_body.data())
                    .await
                    .map_err(|_| anyhow::anyhow!("Upstream H2 stream chunk timed out after 30s"))?;
                match chunk {
                    Some(res) => {
                        let data = res?;
                        let is_eos = upstream_resp_body.is_end_stream();
                        let _ = upstream_resp_body
                            .flow_control()
                            .release_capacity(data.len());
                        client_send_body.send_data(data, is_eos)?;
                        if is_eos {
                            break;
                        }
                    }
                    None => break,
                }
            }

            // Forward response trailers (e.g. grpc-status, grpc-message)
            let trailers = tokio::time::timeout(STREAM_CHUNK_TIMEOUT, upstream_resp_body.trailers())
                .await
                .map_err(|_| anyhow::anyhow!("Upstream H2 response trailers timed out"))??;
            if let Some(trailers) = trailers {
                client_send_body.send_trailers(trailers)?;
            }
        }

        Ok(())
    }
}
