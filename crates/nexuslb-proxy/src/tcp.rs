use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, trace};

use nexuslb_core::backend::Backend;
use nexuslb_metrics::WorkerMetrics;
use nexuslb_network::{BufferPool, SpliceEngine};

struct ConnGuard(Arc<Backend>);
impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.0.stats().dec_active_connections();
    }
}

/// Maximum duration for a TCP/WebSocket session before forced termination.
/// Prevents indefinite connection slot consumption from idle or malicious sessions.
const MAX_TCP_SESSION_DURATION: std::time::Duration = std::time::Duration::from_secs(3600); // 1 hour

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
        let _guard = ConnGuard(backend.clone());

        let res = match tokio::time::timeout(
            MAX_TCP_SESSION_DURATION,
            splice_engine.splice_bidirectional(&mut client, &mut upstream),
        )
        .await
        {
            Ok(r) => r,
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "TCP splice session exceeded max duration (1h)",
            )),
        };

        drop(_guard);

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
        let _guard = ConnGuard(backend.clone());

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

        let copy_fut = async {
            tokio::try_join!(client_to_upstream, upstream_to_client)
        };

        let res = match tokio::time::timeout(MAX_TCP_SESSION_DURATION, copy_fut).await {
            Ok(r) => r,
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "TCP proxy session exceeded max duration (1h)",
            )),
        };

        drop(_guard);

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
