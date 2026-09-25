use std::sync::Arc;
use std::time::{Duration, Instant};

use nexuslb_core::backend::Backend;
use nexuslb_core::types::{BackendAddress, BackendId, BackendState, CircuitState, Protocol};
use nexuslb_health::circuit::{CircuitBreaker, CircuitBreakerConfig};

#[test]
fn test_circuit_breaker_trip_and_recovery() {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let backend = Arc::new(Backend::new(
        BackendId::new(1),
        "test-backend",
        BackendAddress::new(addr),
        100,
        Protocol::Http1,
        None,
    ));
    backend.set_state(BackendState::Up);

    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 2,
        cool_down_duration: Duration::from_millis(50),
        half_open_max_probes: 2,
    };
    let cb = CircuitBreaker::new(config);

    assert_eq!(backend.circuit_state(), CircuitState::Closed);

    // Record failures up to threshold
    backend.stats().record_error();
    cb.on_failure(&backend);
    assert_eq!(backend.circuit_state(), CircuitState::Closed);

    backend.stats().record_error();
    cb.on_failure(&backend);
    assert_eq!(backend.circuit_state(), CircuitState::Closed);

    backend.stats().record_error();
    cb.on_failure(&backend);
    // 3 consecutive failures: tripped!
    assert_eq!(backend.circuit_state(), CircuitState::Open);

    // Check cool down transition to HalfOpen
    let created = Instant::now();
    std::thread::sleep(Duration::from_millis(60));
    cb.maybe_half_open(&backend, Instant::now(), created);
    assert_eq!(backend.circuit_state(), CircuitState::HalfOpen);

    // Record 2 consecutive successes during HalfOpen
    backend
        .stats()
        .record_success(Duration::from_millis(1), 10, 10);
    cb.on_success(&backend);
    assert_eq!(backend.circuit_state(), CircuitState::HalfOpen);

    backend
        .stats()
        .record_success(Duration::from_millis(1), 10, 10);
    cb.on_success(&backend);
    // Recovered to Closed!
    assert_eq!(backend.circuit_state(), CircuitState::Closed);
}
