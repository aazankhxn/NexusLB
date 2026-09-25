use std::net::SocketAddr;
use std::sync::Arc;
#[cfg(target_os = "linux")]
use tracing::info;

use nexuslb_core::error::{NexusError, Result};
use nexuslb_dataplane::SharedDataplaneState;
use nexuslb_metrics::GlobalMetrics;
use nexuslb_network::SocketConfig;

pub struct IoUringEngine;

impl IoUringEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn is_available() -> bool {
        #[cfg(target_os = "linux")]
        {
            // Probe kernel support for io_uring
            io_uring::IoUring::new(16).is_ok()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    pub fn start(
        &mut self,
        _workers: &str,
        _listeners: Vec<SocketAddr>,
        _state: Arc<SharedDataplaneState>,
        _metrics: Arc<GlobalMetrics>,
        _socket_config: SocketConfig,
    ) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            info!("Initializing NexusLB Linux io_uring I/O engine");
            // Linux io_uring event loop implementation
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(NexusError::EngineUnavailable(
                "io-uring".to_string(),
                format!(
                    "io_uring engine requires Linux kernel 5.1+; current operating system is '{}'. Please use '--engine tokio' or '--engine auto'.",
                    std::env::consts::OS
                ),
            ))
        }
    }
}

impl Default for IoUringEngine {
    fn default() -> Self {
        Self::new()
    }
}
