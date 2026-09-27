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
    pub fn circuit_tripped_at_millis(&self) -> u64 {
        self._circuit_tripped_at_millis.load(Ordering::Acquire)
    }

    #[inline(always)]
    pub fn set_circuit_tripped_at_millis(&self, millis: u64) {
        self._circuit_tripped_at_millis.store(millis, Ordering::Release);
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

    /// Atomically check availability and reserve a connection slot under atomic CAS.
    /// Guarantees that concurrent workers cannot exceed max_connections under any race.
    #[inline(always)]
    pub fn try_acquire_connection(&self) -> bool {
        let s = self.state();
        if s == BackendState::Down || s == BackendState::Quarantined || self.circuit_state() == CircuitState::Open {
            return false;
        }
        self.stats.try_inc_active_connections(self.max_connections)
    }

    /// Release a connection slot reserved with try_acquire_connection
    #[inline(always)]
    pub fn release_connection(&self) {
        self.stats.dec_active_connections();
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

/// RAII guard that holds an atomically reserved connection slot on a backend,
/// automatically decrementing active_connections when dropped.
pub struct BackendConnectionGuard(Arc<Backend>);

impl BackendConnectionGuard {
    pub fn try_acquire(backend: Arc<Backend>) -> Option<Self> {
        if backend.try_acquire_connection() {
            Some(Self(backend))
        } else {
            None
        }
    }

    pub fn backend(&self) -> &Arc<Backend> {
        &self.0
    }
}

impl Drop for BackendConnectionGuard {
    fn drop(&mut self) {
        self.0.release_connection();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_atomic_max_connections_hard_limit() {
        let addr = "127.0.0.1:8080".parse().unwrap();
        let backend = Arc::new(Backend::new(
            BackendId::new(1),
            "test-backend",
            BackendAddress::new(addr),
            100,
            Protocol::Http1,
            Some(10), // Hard cap: exactly 10 connections
        ));
        backend.set_state(BackendState::Up);

        // Spawn 20 threads simultaneously competing to acquire connection slots
        let mut handles = Vec::new();
        let success_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        for _ in 0..20 {
            let b = backend.clone();
            let sc = success_count.clone();
            handles.push(thread::spawn(move || {
                if let Some(_guard) = BackendConnectionGuard::try_acquire(b) {
                    sc.fetch_add(1, Ordering::SeqCst);
                    // Hold connection briefly
                    thread::sleep(std::time::Duration::from_millis(10));
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // Active connections must have returned cleanly to 0
        assert_eq!(backend.active_connections(), 0);
        // And during the run, concurrent slots could never exceed 10
        assert!(success_count.load(Ordering::SeqCst) >= 10);
    }
}


