use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use nexuslb_core::error::Result;
use nexuslb_dataplane::SharedDataplaneState;
use nexuslb_metrics::GlobalMetrics;
use nexuslb_network::SocketConfig;
use nexuslb_runtime::RuntimeManager;

pub struct TokioEngine {
    manager: Option<RuntimeManager>,
}

impl TokioEngine {
    pub fn new() -> Self {
        Self { manager: None }
    }

    pub fn is_available() -> bool {
        true
    }

    pub fn start(
        &mut self,
        workers: &str,
        listeners: Vec<SocketAddr>,
        state: Arc<SharedDataplaneState>,
        metrics: Arc<GlobalMetrics>,
        socket_config: SocketConfig,
        max_connections: usize,
    ) -> Result<()> {
        info!("Initializing NexusLB Tokio I/O engine");
        let mut manager = RuntimeManager::new(
            workers,
            listeners,
            state,
            metrics,
            socket_config,
            max_connections,
        );
        manager.start();
        self.manager = Some(manager);
        Ok(())
    }

    pub fn shutdown(&mut self) {
        if let Some(manager) = self.manager.take() {
            manager.shutdown();
        }
    }
}

impl Default for TokioEngine {
    fn default() -> Self {
        Self::new()
    }
}
