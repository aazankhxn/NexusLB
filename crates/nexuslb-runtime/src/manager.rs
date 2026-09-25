use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tokio::sync::watch;
use tracing::info;

use crate::worker::Worker;
use nexuslb_dataplane::SharedDataplaneState;
use nexuslb_metrics::GlobalMetrics;
use nexuslb_network::SocketConfig;

pub struct RuntimeManager {
    num_workers: usize,
    listen_addrs: Vec<SocketAddr>,
    state: Arc<SharedDataplaneState>,
    metrics: Arc<GlobalMetrics>,
    socket_config: SocketConfig,
    shutdown_tx: watch::Sender<bool>,
    handles: Vec<JoinHandle<()>>,
}

impl RuntimeManager {
    pub fn new(
        workers_spec: &str,
        listen_addrs: Vec<SocketAddr>,
        state: Arc<SharedDataplaneState>,
        metrics: Arc<GlobalMetrics>,
        socket_config: SocketConfig,
    ) -> Self {
        let num_workers = match workers_spec.trim().to_ascii_lowercase().as_str() {
            "auto" => thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
            other => other.parse::<usize>().unwrap_or_else(|_| {
                thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4)
            }),
        };

        let (shutdown_tx, _) = watch::channel(false);

        Self {
            num_workers,
            listen_addrs,
            state,
            metrics,
            socket_config,
            shutdown_tx,
            handles: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        info!(
            workers = self.num_workers,
            listeners = ?self.listen_addrs,
            "Starting NexusLB per-core runtime"
        );

        for id in 0..self.num_workers {
            let listen_addrs = self.listen_addrs.clone();
            let state = self.state.clone();
            let worker_metrics = self.metrics.worker(id);
            let socket_config = self.socket_config;
            let shutdown_rx = self.shutdown_tx.subscribe();

            let handle = thread::Builder::new()
                .name(format!("nexuslb-worker-{}", id))
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to build worker Tokio runtime");

                    rt.block_on(async {
                        let worker = Worker {
                            id,
                            listen_addrs,
                            state,
                            metrics: worker_metrics,
                            socket_config,
                        };
                        worker.run(shutdown_rx).await;
                    });
                })
                .expect("Failed to spawn worker thread");

            self.handles.push(handle);
        }
    }

    pub fn shutdown(self) {
        info!("Initiating runtime shutdown");
        let _ = self.shutdown_tx.send(true);
        for handle in self.handles {
            let _ = handle.join();
        }
        info!("All workers stopped cleanly");
    }

    pub fn num_workers(&self) -> usize {
        self.num_workers
    }
}
