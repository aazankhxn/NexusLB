use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;

use crate::stats::{AtomicBackendStats, BackendStatsSnapshot};
use crate::types::{BackendAddress, BackendId, BackendState, CircuitState, Protocol};

/// Representation of an upstream backend server in NexusLB
#[derive(Debug)]
pub struct Backend {
    id: BackendId,
    name: String,
    address: BackendAddress,
    weight: u32,
    state: AtomicU8,
    circuit: AtomicU8,
    protocol: Protocol,
    max_connections: Option<u64>,
    stats: Arc<AtomicBackendStats>,
    _circuit_tripped_at_millis: AtomicU64,
    metadata: HashMap<String, String>,
}

impl Backend {
    pub fn new(
        id: BackendId,
        name: impl Into<String>,
        address: BackendAddress,
        weight: u32,
        protocol: Protocol,
        max_connections: Option<u64>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            address,
            weight: if weight == 0 { 1 } else { weight },
            state: AtomicU8::new(BackendState::Starting as u8),
            circuit: AtomicU8::new(CircuitState::Closed as u8),
            protocol,
            max_connections,
            stats: Arc::new(AtomicBackendStats::new()),
            _circuit_tripped_at_millis: AtomicU64::new(0),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    #[inline(always)]
    pub fn id(&self) -> BackendId {
        self.id
    }

    #[inline(always)]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline(always)]
    pub fn address(&self) -> &BackendAddress {
        &self.address
    }

    #[inline(always)]
    pub fn socket_addr(&self) -> SocketAddr {
        self.address.addr
    }

    #[inline(always)]
    pub fn weight(&self) -> u32 {
        self.weight
    }

    #[inline(always)]
    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    #[inline(always)]
    pub fn max_connections(&self) -> Option<u64> {
        self.max_connections
    }

    #[inline(always)]
    pub fn stats(&self) -> &Arc<AtomicBackendStats> {
        &self.stats
    }

    #[inline(always)]
    pub fn state(&self) -> BackendState {
        match self.state.load(Ordering::Acquire) {
            0 => BackendState::Starting,
            1 => BackendState::Up,
            2 => BackendState::Down,
            3 => BackendState::Draining,
            4 => BackendState::Quarantined,
            _ => BackendState::Down,
        }
    }

    #[inline(always)]
    pub fn set_state(&self, new_state: BackendState) {
        let val = match new_state {
            BackendState::Starting => 0,
            BackendState::Up => 1,
            BackendState::Down => 2,
            BackendState::Draining => 3,
            BackendState::Quarantined => 4,
        };
        self.state.store(val, Ordering::Release);
    }

    #[inline(always)]
    pub fn circuit_state(&self) -> CircuitState {
        match self.circuit.load(Ordering::Acquire) {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Open,
        }
    }

    #[inline(always)]
    pub fn set_circuit_state(&self, state: CircuitState) {
        let val = match state {
            CircuitState::Closed => 0,
            CircuitState::Open => 1,
            CircuitState::HalfOpen => 2,
        };
        self.circuit.store(val, Ordering::Release);
    }

    #[inline(always)]
    pub fn is_available(&self) -> bool {
        let s = self.state();
        let c = self.circuit_state();

        if !s.is_routable() {
            return false;
        }

        if c == CircuitState::Open {
            return false;
        }

        if let Some(max_conn) = self.max_connections {
            if self.stats.active_connections() >= max_conn {
                return false;
            }
        }

        true
    }

    #[inline(always)]
    pub fn active_connections(&self) -> u64 {
        self.stats.active_connections()
    }

    pub fn snapshot(&self) -> BackendSnapshot {
        BackendSnapshot {
            id: self.id,
            name: self.name.clone(),
            address: self.address.to_string(),
            weight: self.weight,
            state: self.state(),
            circuit: self.circuit_state(),
            protocol: self.protocol,
            max_connections: self.max_connections,
            stats: self.stats.snapshot(),
            metadata: self.metadata.clone(),
        }
    }
}

/// Point-in-time snapshot of backend for serialization/admin API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSnapshot {
    pub id: BackendId,
    pub name: String,
    pub address: String,
    pub weight: u32,
    pub state: BackendState,
    pub circuit: CircuitState,
    pub protocol: Protocol,
    pub max_connections: Option<u64>,
    pub stats: BackendStatsSnapshot,
    pub metadata: HashMap<String, String>,
}
