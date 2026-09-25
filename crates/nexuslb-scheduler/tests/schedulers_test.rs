use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use nexuslb_core::backend::Backend;
use nexuslb_core::types::{AlgorithmType, BackendAddress, BackendId, BackendState, Protocol};
use nexuslb_scheduler::adaptive::AdaptiveScheduler;
use nexuslb_scheduler::consistent_hash::ConsistentHashScheduler;
use nexuslb_scheduler::ewma_latency::EwmaLatencyScheduler;
use nexuslb_scheduler::factory::create_scheduler;
use nexuslb_scheduler::ip_hash::IpHashScheduler;
use nexuslb_scheduler::least_connections::LeastConnectionsScheduler;
use nexuslb_scheduler::least_latency::LeastLatencyScheduler;
use nexuslb_scheduler::power_of_two::PowerOfTwoChoicesScheduler;
use nexuslb_scheduler::random::RandomScheduler;
use nexuslb_scheduler::round_robin::RoundRobinScheduler;
use nexuslb_scheduler::traits::{Scheduler, SelectionContext};
use nexuslb_scheduler::weighted_round_robin::WeightedRoundRobinScheduler;

fn make_backend(id: u64, name: &str, weight: u32) -> Arc<Backend> {
    let addr = format!("127.0.0.1:{}", 8000 + id)
        .parse::<SocketAddr>()
        .unwrap();
    let b = Arc::new(Backend::new(
        BackendId::new(id),
        name,
        BackendAddress::new(addr),
        weight,
        Protocol::Http1,
        None,
    ));
    b.set_state(BackendState::Up);
    b
}

#[test]
fn test_round_robin() {
    let b1 = make_backend(1, "b1", 100);
    let b2 = make_backend(2, "b2", 100);
    let backends = vec![b1.clone(), b2.clone()];

    let rr = RoundRobinScheduler::new();
    let ctx = SelectionContext::default();

    assert_eq!(rr.select(&backends, &ctx).unwrap().id(), b1.id());
    assert_eq!(rr.select(&backends, &ctx).unwrap().id(), b2.id());
    assert_eq!(rr.select(&backends, &ctx).unwrap().id(), b1.id());
}

#[test]
fn test_weighted_round_robin() {
    let b1 = make_backend(1, "b1", 3);
    let b2 = make_backend(2, "b2", 1);
    let backends = vec![b1.clone(), b2.clone()];

    let wrr = WeightedRoundRobinScheduler::new();
    let ctx = SelectionContext::default();

    let mut count_b1 = 0;
    let mut count_b2 = 0;

    for _ in 0..40 {
        let picked = wrr.select(&backends, &ctx).unwrap();
        if picked.id() == b1.id() {
            count_b1 += 1;
        } else {
            count_b2 += 1;
        }
    }

    assert_eq!(count_b1, 30);
    assert_eq!(count_b2, 10);
}

#[test]
fn test_least_connections() {
    let b1 = make_backend(1, "b1", 100);
    let b2 = make_backend(2, "b2", 100);

    b1.stats().inc_active_connections();
    b1.stats().inc_active_connections();
    b2.stats().inc_active_connections();

    let backends = vec![b1.clone(), b2.clone()];
    let lc = LeastConnectionsScheduler::new();
    let ctx = SelectionContext::default();

    let picked = lc.select(&backends, &ctx).unwrap();
    assert_eq!(picked.id(), b2.id(), "b2 has 1 conn, b1 has 2");
}

#[test]
fn test_random_scheduler() {
    let b1 = make_backend(1, "b1", 100);
    let b2 = make_backend(2, "b2", 100);
    let backends = vec![b1.clone(), b2.clone()];

    let rand = RandomScheduler::new();
    let ctx = SelectionContext::default();

    let picked = rand.select(&backends, &ctx);
    assert!(picked.is_some());
}

#[test]
fn test_ip_hash_affinity() {
    let b1 = make_backend(1, "b1", 100);
    let b2 = make_backend(2, "b2", 100);
    let b3 = make_backend(3, "b3", 100);
    let backends = vec![b1, b2, b3];

    let ip_hash = IpHashScheduler::new();
    let ctx_client_a = SelectionContext::with_ip("192.168.1.50".parse().unwrap());
    let ctx_client_b = SelectionContext::with_ip("10.0.0.12".parse().unwrap());

    let pick_a1 = ip_hash.select(&backends, &ctx_client_a).unwrap();
    let pick_a2 = ip_hash.select(&backends, &ctx_client_a).unwrap();
    assert_eq!(
        pick_a1.id(),
        pick_a2.id(),
        "Same client IP must route to same backend"
    );

    let pick_b1 = ip_hash.select(&backends, &ctx_client_b).unwrap();
    let pick_b2 = ip_hash.select(&backends, &ctx_client_b).unwrap();
    assert_eq!(
        pick_b1.id(),
        pick_b2.id(),
        "Same client IP must consistently hash"
    );
}

#[test]
fn test_consistent_hashing() {
    let b1 = make_backend(1, "b1", 100);
    let b2 = make_backend(2, "b2", 100);
    let backends = vec![b1, b2];

    let ch = ConsistentHashScheduler::new();
    let ctx1 = SelectionContext::with_key("session-token-xyz");
    let ctx2 = SelectionContext::with_key("session-token-xyz");

    let pick1 = ch.select(&backends, &ctx1).unwrap();
    let pick2 = ch.select(&backends, &ctx2).unwrap();
    assert_eq!(
        pick1.id(),
        pick2.id(),
        "Identical key must hash to same virtual node"
    );
}

#[test]
fn test_power_of_two_choices() {
    let b1 = make_backend(1, "b1", 100);
    let b2 = make_backend(2, "b2", 100);
    b1.stats().inc_active_connections();
    b1.stats().inc_active_connections();
    b1.stats().inc_active_connections();

    let backends = vec![b1.clone(), b2.clone()];
    let p2c = PowerOfTwoChoicesScheduler::new();
    let ctx = SelectionContext::default();

    let picked = p2c.select(&backends, &ctx).unwrap();
    assert_eq!(picked.id(), b2.id(), "P2C chooses less loaded backend");
}

#[test]
fn test_least_latency_and_ewma() {
    let b1 = make_backend(1, "b1", 100);
    let b2 = make_backend(2, "b2", 100);

    b1.stats()
        .record_success(Duration::from_millis(50), 100, 100);
    b2.stats()
        .record_success(Duration::from_millis(5), 100, 100);

    let backends = vec![b1.clone(), b2.clone()];
    let ll = LeastLatencyScheduler::new();
    let ewma = EwmaLatencyScheduler::new();
    let ctx = SelectionContext::default();

    assert_eq!(ll.select(&backends, &ctx).unwrap().id(), b2.id());
    assert_eq!(ewma.select(&backends, &ctx).unwrap().id(), b2.id());
}

#[test]
fn test_adaptive_scheduler_scoring() {
    // Backend A: connections: 10, latency: 5ms, errors: 0
    let b_a = make_backend(1, "backend-a", 100);
    for _ in 0..10 {
        b_a.stats().inc_active_connections();
    }
    b_a.stats()
        .record_success(Duration::from_millis(5), 100, 100);

    // Backend B: connections: 3, latency: 80ms, errors: 4 consecutive
    let b_b = make_backend(2, "backend-b", 100);
    for _ in 0..3 {
        b_b.stats().inc_active_connections();
    }
    b_b.stats()
        .record_success(Duration::from_millis(80), 100, 100);
    for _ in 0..4 {
        b_b.stats().record_error();
    }

    // Backend C: connections: 8, latency: 8ms, errors: 0
    let b_c = make_backend(3, "backend-c", 100);
    for _ in 0..8 {
        b_c.stats().inc_active_connections();
    }
    b_c.stats()
        .record_success(Duration::from_millis(8), 100, 100);

    let backends = vec![b_a.clone(), b_b.clone(), b_c.clone()];
    let adaptive = AdaptiveScheduler::new();
    let ctx = SelectionContext::default();

    let picked = adaptive.select(&backends, &ctx).unwrap();
    // B must NOT be chosen despite having fewer connections because of high latency & errors
    assert_ne!(
        picked.id(),
        b_b.id(),
        "Adaptive scheduler must penalize bad backend B"
    );
    assert!(picked.id() == b_a.id() || picked.id() == b_c.id());
}

#[test]
fn test_factory_creates_all_ten_algorithms() {
    let types = [
        AlgorithmType::RoundRobin,
        AlgorithmType::WeightedRoundRobin,
        AlgorithmType::LeastConnections,
        AlgorithmType::Random,
        AlgorithmType::IpHash,
        AlgorithmType::ConsistentHash,
        AlgorithmType::PowerOfTwoChoices,
        AlgorithmType::LeastLatency,
        AlgorithmType::EwmaLatency,
        AlgorithmType::Adaptive,
    ];

    for t in types {
        let s = create_scheduler(t);
        assert_eq!(s.algorithm(), t);
    }
}
