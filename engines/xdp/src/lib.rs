use nexuslb_core::error::{NexusError, Result};
use nexuslb_dataplane::SharedDataplaneState;
use nexuslb_metrics::GlobalMetrics;
use nexuslb_network::SocketConfig;
use std::net::SocketAddr;
use std::sync::Arc;

pub struct XdpEngine;

impl XdpEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn is_available() -> bool {
        false // AF_XDP requires loaded eBPF program, Linux kernel, and CAP_NET_ADMIN
    }

    pub fn start(
        &mut self,
        _workers: &str,
        _listeners: Vec<SocketAddr>,
        _state: Arc<SharedDataplaneState>,
        _metrics: Arc<GlobalMetrics>,
        _socket_config: SocketConfig,
    ) -> Result<()> {
        Err(NexusError::EngineUnavailable(
            "xdp".to_string(),
            format!(
                "XDP / AF_XDP engine requires Linux with CAP_NET_ADMIN privileges and a loaded BPF filter; current OS is '{}'. Please use '--engine tokio' or '--engine auto'.",
                std::env::consts::OS
            ),
        ))
    }
}

impl Default for XdpEngine {
    fn default() -> Self {
        Self::new()
    }
}
