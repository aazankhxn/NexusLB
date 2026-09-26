use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tokio::sync::watch;
use tracing::info;

use nexuslb_dataplane::{DataplanePipeline, SharedDataplaneState};
use nexuslb_metrics::GlobalMetrics;
use nexuslb_network::{create_listener, SocketConfig};

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
            "Starting NexusLB high-throughput work-stealing runtime"
        );

        let listen_addrs = self.listen_addrs.clone();
        let state = self.state.clone();
        let metrics = self.metrics.clone();
        let socket_config = self.socket_config;
        let shutdown_rx = self.shutdown_tx.subscribe();
        let num_workers = self.num_workers;

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // Already inside a Tokio runtime: spawn listener tasks directly onto it!
            handle.spawn(async move {
                Self::run_listeners(
                    listen_addrs,
                    socket_config,
                    shutdown_rx,
                    state,
                    metrics,
                    num_workers,
                )
                .await;
            });
            return;
        }

        let handle = thread::Builder::new()
            .name("nexuslb-core-runtime".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(num_workers)
                    .enable_all()
                    .build()
                    .expect("Failed to build multi-thread Tokio runtime");

                rt.block_on(async move {
                    Self::run_listeners(
                        listen_addrs,
                        socket_config,
                        shutdown_rx,
                        state,
                        metrics,
                        num_workers,
                    )
                    .await;
                });
            })
            .expect("Failed to spawn runtime thread");

        self.handles.push(handle);
    }

    async fn run_listeners(
        listen_addrs: Vec<SocketAddr>,
        socket_config: SocketConfig,
        shutdown_rx: watch::Receiver<bool>,
        state: Arc<SharedDataplaneState>,
        metrics: Arc<GlobalMetrics>,
        num_workers: usize,
    ) {
        let mut listeners = Vec::new();
        for addr in &listen_addrs {
            let count = if socket_config.reuse_port {
                num_workers.max(1)
            } else {
                1
            };
            for _ in 0..count {
                match create_listener(*addr, &socket_config) {
                    Ok(l) => listeners.push(l),
                    Err(e) => {
                        tracing::error!(address = %addr, error = %e, "Failed to bind listener");
                        break;
                    }
                }
            }
        }

        // Shared shutdown flag — checked with Relaxed load instead of tokio::select! overhead
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let mut accept_tasks = Vec::new();

        // Spawn shutdown watcher task
        {
            let flag = shutdown_flag.clone();
            let mut rx = shutdown_rx;
            tokio::spawn(async move {
                let _ = rx.changed().await;
                if *rx.borrow() {
                    flag.store(true, Ordering::Release);
                }
            });
        }

        for (idx, listener) in listeners.into_iter().enumerate() {
            let s = state.clone();
            let worker_idx = idx % num_workers;
            let m_worker = metrics.worker(worker_idx);
            let flag = shutdown_flag.clone();

            accept_tasks.push(tokio::spawn(async move {
                loop {
                    // Fast shutdown check: single Relaxed atomic load (~1ns) instead of
                    // tokio::select! macro overhead (~30-50ns per iteration with waker registration)
                    if flag.load(Ordering::Relaxed) {
                        break;
                    }

                    match listener.accept().await {
                        Ok((stream, client_addr)) => {
                            let _ = stream.set_nodelay(true);
                            let s_clone = s.clone();
                            let m_clone = m_worker.clone();

                            tokio::spawn(async move {
                                DataplanePipeline::process_connection(
                                    stream,
                                    client_addr,
                                    s_clone,
                                    m_clone,
                                )
                                .await;
                            });
                        }
                        Err(e) => {
                            tracing::trace!(error = %e, "Accept error");
                        }
                    }
                }
            }));
        }

        for task in accept_tasks {
            let _ = task.await;
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
