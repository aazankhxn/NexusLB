use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, trace};

use nexuslb_core::backend::Backend;
use nexuslb_metrics::WorkerMetrics;
use nexuslb_network::BufferPool;

pub struct TcpProxy;

impl TcpProxy {
    pub async fn forward(
        mut client: TcpStream,
        mut upstream: TcpStream,
        backend: Arc<Backend>,
        metrics: Arc<WorkerMetrics>,
        buffer_pool: BufferPool,
    ) -> std::io::Result<()> {
        let start = Instant::now();
        backend.stats().inc_active_connections();
        metrics.inc_connections();

        let (mut client_read, mut client_write) = client.split();
        let (mut upstream_read, mut upstream_write) = upstream.split();

        let pool_c2u = buffer_pool.clone();
        let pool_u2c = buffer_pool.clone();

        let metrics_c2u = metrics.clone();
        let metrics_u2c = metrics.clone();

        let client_to_upstream = async move {
            let mut buf = pool_c2u.acquire();
            let mut total_bytes = 0u64;
            loop {
                let n = client_read.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                upstream_write.write_all(&buf[..n]).await?;
                total_bytes += n as u64;
                metrics_c2u.add_bytes_received(n as u64);
            }
            upstream_write.shutdown().await?;
            Ok::<u64, std::io::Error>(total_bytes)
        };

        let upstream_to_client = async move {
            let mut buf = pool_u2c.acquire();
            let mut total_bytes = 0u64;
            loop {
                let n = upstream_read.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                client_write.write_all(&buf[..n]).await?;
                total_bytes += n as u64;
                metrics_u2c.add_bytes_sent(n as u64);
            }
            client_write.shutdown().await?;
            Ok::<u64, std::io::Error>(total_bytes)
        };

        let res = tokio::try_join!(client_to_upstream, upstream_to_client);

        backend.stats().dec_active_connections();
        metrics.dec_connections();

        match res {
            Ok((bytes_in, bytes_out)) => {
                let duration = start.elapsed();
                backend
                    .stats()
                    .record_success(duration, bytes_in, bytes_out);
                metrics.record_latency(duration);
                trace!(
                    backend = %backend.name(),
                    bytes_in,
                    bytes_out,
                    duration_ms = duration.as_millis(),
                    "TCP proxy session completed cleanly"
                );
                Ok(())
            }
            Err(e) => {
                backend.stats().record_error();
                metrics.inc_backend_errors();
                debug!(backend = %backend.name(), error = %e, "TCP proxy session ended with error");
                Err(e)
            }
        }
    }
}
