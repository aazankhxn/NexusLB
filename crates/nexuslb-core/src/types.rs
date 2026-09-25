use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique identifier for a backend instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BackendId(pub u64);

impl BackendId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "backend-{}", self.0)
    }
}

/// Operational state of a backend server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BackendState {
    #[default]
    Starting,
    Up,
    Down,
    Draining,
    Quarantined,
}

impl BackendState {
    #[inline(always)]
    pub fn is_routable(self) -> bool {
        matches!(self, BackendState::Up)
    }

    #[inline(always)]
    pub fn is_draining(self) -> bool {
        matches!(self, BackendState::Draining)
    }

    #[inline(always)]
    pub fn is_healthy(self) -> bool {
        matches!(self, BackendState::Up | BackendState::Starting)
    }
}

impl fmt::Display for BackendState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendState::Starting => write!(f, "STARTING"),
            BackendState::Up => write!(f, "UP"),
            BackendState::Down => write!(f, "DOWN"),
            BackendState::Draining => write!(f, "DRAINING"),
            BackendState::Quarantined => write!(f, "QUARANTINED"),
        }
    }
}

/// Circuit breaker state for a backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CircuitState {
    #[default]
    Closed, // Normal operation: all traffic allowed
    Open,     // Tripped: traffic diverted/dropped
    HalfOpen, // Probing: limited traffic to test recovery
}

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "CLOSED"),
            CircuitState::Open => write!(f, "OPEN"),
            CircuitState::HalfOpen => write!(f, "HALF_OPEN"),
        }
    }
}

/// Supported transport and application protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[default]
    Tcp,
    Http1,
    Http2,
    Grpc,
    WebSocket,
    Tls,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "tcp"),
            Protocol::Http1 => write!(f, "http1"),
            Protocol::Http2 => write!(f, "http2"),
            Protocol::Grpc => write!(f, "grpc"),
            Protocol::WebSocket => write!(f, "websocket"),
            Protocol::Tls => write!(f, "tls"),
        }
    }
}

/// Load balancing algorithm specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlgorithmType {
    RoundRobin,
    WeightedRoundRobin,
    LeastConnections,
    Random,
    IpHash,
    ConsistentHash,
    PowerOfTwoChoices,
    LeastLatency,
    EwmaLatency,
    #[default]
    Adaptive,
}

impl fmt::Display for AlgorithmType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlgorithmType::RoundRobin => write!(f, "round_robin"),
            AlgorithmType::WeightedRoundRobin => write!(f, "weighted_round_robin"),
            AlgorithmType::LeastConnections => write!(f, "least_connections"),
            AlgorithmType::Random => write!(f, "random"),
            AlgorithmType::IpHash => write!(f, "ip_hash"),
            AlgorithmType::ConsistentHash => write!(f, "consistent_hash"),
            AlgorithmType::PowerOfTwoChoices => write!(f, "power_of_two_choices"),
            AlgorithmType::LeastLatency => write!(f, "least_latency"),
            AlgorithmType::EwmaLatency => write!(f, "ewma_latency"),
            AlgorithmType::Adaptive => write!(f, "adaptive"),
        }
    }
}

/// Engine selector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum EngineType {
    #[default]
    Auto,
    Tokio,
    IoUring,
    Xdp,
    AfXdp,
}

impl fmt::Display for EngineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineType::Auto => write!(f, "auto"),
            EngineType::Tokio => write!(f, "tokio"),
            EngineType::IoUring => write!(f, "io-uring"),
            EngineType::Xdp => write!(f, "xdp"),
            EngineType::AfXdp => write!(f, "af-xdp"),
        }
    }
}

/// Backend address descriptor
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BackendAddress {
    pub addr: SocketAddr,
    pub hostname: Option<String>,
}

impl BackendAddress {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            hostname: None,
        }
    }

    pub fn with_hostname(addr: SocketAddr, hostname: impl Into<String>) -> Self {
        Self {
            addr,
            hostname: Some(hostname.into()),
        }
    }
}

impl fmt::Display for BackendAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref host) = self.hostname {
            write!(f, "{}:{}", host, self.addr.port())
        } else {
            write!(f, "{}", self.addr)
        }
    }
}
