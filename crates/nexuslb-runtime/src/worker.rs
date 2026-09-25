use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{debug, error, info, trace};

use crate::affinity::set_core_affinity;
use nexuslb_dataplane::{DataplanePipeline, SharedDataplaneState};
use nexuslb_metrics::WorkerMetrics;
use nexuslb_network::{create_listener, SocketConfig};

pub struct Worker {
    pub id: usize,
    pub listen_addrs: Vec<SocketAddr>,
    pub state: Arc<SharedDataplaneState>,
    pub metrics: Arc<WorkerMetrics>,
    pub socket_config: SocketConfig,
}

impl Worker {
    pub async fn run(self, shutdown_rx: watch::Receiver<bool>) {
        set_core_affinity(self.id);
        debug!(worker_id = self.id, "Worker event loop started");

        let mut listeners = Vec::new();
        for addr in &self.listen_addrs {
            match create_listener(*addr, &self.socket_config) {
                Ok(l) => listeners.push(Arc::new(l)),
                Err(e) => {
                    error!(
                        worker_id = self.id,
                        address = %addr,
                        error = %e,
                        "Failed to bind listener for worker"
                    );
                    return;
                }
            }
        }

        if listeners.is_empty() {
            error!(worker_id = self.id, "No listeners configured for worker");
            return;
        }

        // Spawn accept task for each listener
        let mut accept_tasks = Vec::new();

        for listener in listeners {
            let state = self.state.clone();
            let metrics = self.metrics.clone();
            let mut rx = shutdown_rx.clone();
            let worker_id = self.id;

            accept_tasks.push(tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = rx.changed() => {
                            if *rx.borrow() {
                                break;
                            }
                        }
                        res = listener.accept() => {
                            match res {
                                Ok((stream, client_addr)) => {
                                    let s = state.clone();
                                    let m = metrics.clone();
                                    tokio::spawn(async move {
                                        DataplanePipeline::process_connection(
                                            stream,
                                            client_addr,
                                            s,
                                            m,
                                        ).await;
                                    });
                                }
                                Err(e) => {
                                    trace!(worker_id, error = %e, "Accept error");
                                }
                            }
                        }
                    }
                }
            }));
        }

        for task in accept_tasks {
            let _ = task.await;
        }

        info!(worker_id = self.id, "Worker stopped cleanly");
    }
}
