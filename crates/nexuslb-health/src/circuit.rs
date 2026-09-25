use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::{info, warn};

use nexuslb_core::backend::Backend;
use nexuslb_core::types::CircuitState;

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub cool_down_duration: Duration,
    pub half_open_max_probes: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            cool_down_duration: Duration::from_secs(10),
            half_open_max_probes: 2,
        }
    }
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state_changed_at: AtomicU64,
    half_open_probes: AtomicU32,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state_changed_at: AtomicU64::new(0),
            half_open_probes: AtomicU32::new(0),
        }
    }

    pub fn on_success(&self, backend: &Backend) {
        match backend.circuit_state() {
            CircuitState::Closed => {
                // Already closed, normal state
            }
            CircuitState::HalfOpen => {
                let consecutive = backend.stats().consecutive_successes();
                if consecutive >= self.config.success_threshold {
                    backend.set_circuit_state(CircuitState::Closed);
                    info!(
                        backend_id = %backend.id(),
                        name = %backend.name(),
                        "Circuit breaker recovered -> CLOSED"
                    );
                }
            }
            CircuitState::Open => {
                // Should not happen unless cool down passed and probe succeeded
            }
        }
    }

    pub fn on_failure(&self, backend: &Backend) {
        match backend.circuit_state() {
            CircuitState::Closed => {
                let consecutive = backend.stats().consecutive_errors();
                if consecutive >= self.config.failure_threshold {
                    backend.set_circuit_state(CircuitState::Open);
                    let now_ms = Instant::now().elapsed().as_millis() as u64;
                    self.state_changed_at.store(now_ms, Ordering::Release);
                    warn!(
                        backend_id = %backend.id(),
                        name = %backend.name(),
                        consecutive_errors = consecutive,
                        "Circuit breaker tripped -> OPEN"
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Any error during half-open trips it immediately back to OPEN
                backend.set_circuit_state(CircuitState::Open);
                let now_ms = Instant::now().elapsed().as_millis() as u64;
                self.state_changed_at.store(now_ms, Ordering::Release);
                warn!(
                    backend_id = %backend.id(),
                    name = %backend.name(),
                    "Probe failed during HALF_OPEN -> OPEN"
                );
            }
            CircuitState::Open => {}
        }
    }

    pub fn maybe_half_open(&self, backend: &Backend, now: Instant, created_at: Instant) {
        if backend.circuit_state() == CircuitState::Open {
            let changed_at_ms = self.state_changed_at.load(Ordering::Acquire);
            let current_ms = now.duration_since(created_at).as_millis() as u64;
            if current_ms.saturating_sub(changed_at_ms)
                >= self.config.cool_down_duration.as_millis() as u64
            {
                backend.set_circuit_state(CircuitState::HalfOpen);
                self.half_open_probes.store(0, Ordering::Release);
                info!(
                    backend_id = %backend.id(),
                    name = %backend.name(),
                    "Cool-down elapsed -> testing recovery in HALF_OPEN state"
                );
            }
        }
    }
}
