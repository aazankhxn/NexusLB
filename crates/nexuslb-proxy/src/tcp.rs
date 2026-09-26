use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, trace};

use nexuslb_core::backend::Backend;
use nexuslb_metrics::WorkerMetrics;
use nexuslb_network::{BufferPool, SpliceEngine};

pub struct TcpProxy;

impl TcpProxy {
    pub async fn forward_splice(
        mut client: TcpStream,
        mut upstream: TcpStream,
        backend: Arc<Backend>,
        metrics: Arc<WorkerMetrics>,
        splice_engine: &SpliceEngine,
    ) -> std::io::Result<()> {
        let start = Instant::now();
        backend.stats().inc_active_connections();

        let res = splice_engine
            .splice_bidirectional(&mut client, &mut upstream)
            .await;

        backend.stats().dec_active_connections();

        match res {
            Ok(stats) => {
                let duration = start.elapsed();
                backend.stats().record_success(
                    duration,
                    stats.bytes_client_to_backend,
                    stats.bytes_backend_to_client,
                );
                metrics.record_latency(duration);
                metrics.add_bytes_received(stats.bytes_client_to_backend);
                metrics.add_bytes_sent(stats.bytes_backend_to_client);
                trace!(
                    backend = %backend.name(),
                    bytes_in = stats.bytes_client_to_backend,
                    bytes_out = stats.bytes_backend_to_client,
                    zero_copy = stats.zero_copy_used,
                    duration_ms = duration.as_millis(),
                    "TCP splice proxy session completed cleanly"
                );
                Ok(())
            }
            Err(e) => {
                backend.stats().record_error();
                metrics.inc_backend_errors();
                debug!(backend = %backend.name(), error = %e, "TCP splice proxy session ended with error");
                Err(e)
            }
        }
    }
    pub async fn forward<C, U>(
        client: C,
        upstream: U,
        backend: Arc<Backend>,
        metrics: Arc<WorkerMetrics>,
        buffer_pool: BufferPool,
    ) -> std::io::Result<()>
    where
        C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
        U: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let start = Instant::now();
        backend.stats().inc_active_connections();

        let (mut client_read, mut client_write) = tokio::io::split(client);
        let (mut upstream_read, mut upstream_write) = tokio::io::split(upstream);

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
