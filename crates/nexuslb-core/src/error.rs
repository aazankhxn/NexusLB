use crate::types::BackendId;
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NexusError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("No healthy backend available in pool")]
    NoHealthyBackend,

    #[error("Backend not found: {0}")]
    BackendNotFound(BackendId),

    #[error("Backend {0} is draining and not accepting new connections")]
    BackendDraining(BackendId),

    #[error("Backend {0} circuit breaker is open")]
    CircuitOpen(BackendId),

    #[error("Connection limit reached for backend {0}")]
    ConnectionLimitReached(BackendId),

    #[error("Global connection limit reached")]
    GlobalConnectionLimitReached,

    #[error("Rate limit exceeded for client {0}")]
    RateLimitExceeded(SocketAddr),

    #[error("Connection timed out to backend {0}")]
    BackendTimeout(BackendAddressError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Engine '{0}' is not supported or not available on this platform: {1}")]
    EngineUnavailable(String, String),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Admin API authentication failed")]
    Unauthorized,

    #[error("Routing error: no matching route found for request")]
    NoMatchingRoute,

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct BackendAddressError {
    pub id: BackendId,
    pub addr: SocketAddr,
}

impl std::fmt::Display for BackendAddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.id, self.addr)
    }
}

pub type Result<T> = std::result::Result<T, NexusError>;
