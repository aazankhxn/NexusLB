use bytes::Bytes;
use h2::server;
use http::{Request, Response};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpStream;
use tracing::{debug, error, trace};

use nexuslb_core::backend::Backend;
use nexuslb_metrics::WorkerMetrics;

/// High-performance HTTP/2 and gRPC multiplexed stream proxy
pub struct H2Proxy;

impl H2Proxy {
    /// Accept HTTP/2 client connection and multiplex individual streams to backends
    pub async fn handle_connection(
        client: TcpStream,
        client_addr: SocketAddr,
        backend: Arc<Backend>,
        metrics: Arc<WorkerMetrics>,
    ) -> std::io::Result<()> {
        let mut connection = server::handshake(client).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                format!("H2 handshake error: {:?}", e),
            )
        })?;

        trace!(client = %client_addr, "HTTP/2 connection handshake established");

        while let Some(result) = connection.accept().await {
            let (req, mut respond) = match result {
                Ok(stream) => stream,
                Err(e) => {
                    debug!("H2 accept stream error: {:?}", e);
                    break;
                }
            };

            let backend = backend.clone();
            let metrics = metrics.clone();

            tokio::spawn(async move {
                let start = Instant::now();
                metrics.inc_requests();
                backend.stats().inc_requests();
                backend.stats().inc_active_connections();

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

                backend.stats().dec_active_connections();
            });
        }

        Ok(())
    }

    async fn proxy_stream(
        client_req: Request<h2::RecvStream>,
        client_respond: &mut server::SendResponse<Bytes>,
        backend: Arc<Backend>,
    ) -> anyhow::Result<()> {
        // Connect to upstream backend
        let upstream_stream = TcpStream::connect(backend.socket_addr()).await?;
        let _ = upstream_stream.set_nodelay(true);

        let (mut client_h2, upstream_conn) = h2::client::handshake(upstream_stream).await?;

        // Drive upstream connection in background
        tokio::spawn(async move {
            if let Err(e) = upstream_conn.await {
                debug!("Upstream H2 connection closed: {:?}", e);
            }
        });

        // Forward request headers
        let (parts, mut body_recv) = client_req.into_parts();
        let upstream_req = Request::from_parts(parts, ());
        let is_end_of_stream = body_recv.is_end_stream();

        let (resp_fut, mut upstream_send_body) =
            client_h2.send_request(upstream_req, is_end_of_stream)?;

        // Forward client streaming data frames to upstream
        if !is_end_of_stream {
            while let Some(chunk) = body_recv.data().await {
                let data = chunk?;
                let is_eos = body_recv.is_end_stream();
                let _ = body_recv.flow_control().release_capacity(data.len());
                upstream_send_body.send_data(data, is_eos)?;
            }

            // Forward trailers if present (e.g. gRPC trailers)
            if let Some(trailers) = body_recv.trailers().await? {
                upstream_send_body.send_trailers(trailers)?;
            }
        }

        // Await upstream response headers
        let upstream_resp = resp_fut.await?;
        let (resp_parts, mut upstream_resp_body) = upstream_resp.into_parts();
        let client_resp = Response::from_parts(resp_parts, ());
        let resp_is_eos = upstream_resp_body.is_end_stream();

        let mut client_send_body = client_respond.send_response(client_resp, resp_is_eos)?;

        // Forward upstream streaming response data frames to client
        if !resp_is_eos {
            while let Some(chunk) = upstream_resp_body.data().await {
                let data = chunk?;
                let is_eos = upstream_resp_body.is_end_stream();
                let _ = upstream_resp_body
                    .flow_control()
                    .release_capacity(data.len());
                client_send_body.send_data(data, is_eos)?;
            }

            // Forward response trailers (e.g. grpc-status, grpc-message)
            if let Some(trailers) = upstream_resp_body.trailers().await? {
                client_send_body.send_trailers(trailers)?;
            }
        }

        Ok(())
    }
}
