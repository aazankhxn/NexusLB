use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

use crate::circuit::CircuitBreaker;
use nexuslb_core::backend::Backend;

pub struct PassiveHealthDetector {
    circuit_breaker: Arc<CircuitBreaker>,
}

impl PassiveHealthDetector {
    pub fn new(circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self { circuit_breaker }
    }

    pub fn record_success(
        &self,
        backend: &Backend,
        latency: Duration,
        bytes_in: u64,
        bytes_out: u64,
    ) {
        backend.stats().record_success(latency, bytes_in, bytes_out);
        self.circuit_breaker.on_success(backend);
    }

    pub fn record_failure(&self, backend: &Backend, reason: &str) {
        backend.stats().record_error();
        warn!(
            backend_id = %backend.id(),
            name = %backend.name(),
            reason = %reason,
            consecutive_errors = backend.stats().consecutive_errors(),
            "Passive health check detected backend failure"
        );
        self.circuit_breaker.on_failure(backend);
    }
}
