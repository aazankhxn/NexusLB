use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tracing::{debug, info};

use crate::affinity::set_core_affinity;
use nexuslb_dataplane::{DataplanePipeline, SharedDataplaneState};
use nexuslb_metrics::WorkerMetrics;

pub struct Worker {
    pub id: usize,
    pub state: Arc<SharedDataplaneState>,
    pub metrics: Arc<WorkerMetrics>,
}

impl Worker {
    pub async fn run(
        self,
        mut conn_rx: mpsc::Receiver<(std::net::TcpStream, SocketAddr)>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) {
        set_core_affinity(self.id);
        debug!(worker_id = self.id, "Worker event loop started");

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                }
                Some((std_stream, client_addr)) = conn_rx.recv() => {
                    let s = self.state.clone();
                    let m = self.metrics.clone();
                    tokio::spawn(async move {
                        if let Ok(stream) = tokio::net::TcpStream::from_std(std_stream) {
                            let _ = stream.set_nodelay(true);
                            DataplanePipeline::process_connection(
                                stream,
                                client_addr,
                                s,
                                m,
                            ).await;
                        }
                    });
                }
            }
        }

        info!(worker_id = self.id, "Worker stopped cleanly");
    }
}
