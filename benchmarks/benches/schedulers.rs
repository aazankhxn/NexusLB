use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::{BackendAddress, BackendId, Protocol};
use nexuslb_scheduler::adaptive::AdaptiveScheduler;
use nexuslb_scheduler::consistent_hash::ConsistentHashScheduler;
use nexuslb_scheduler::ewma_latency::EwmaLatencyScheduler;
use nexuslb_scheduler::ip_hash::IpHashScheduler;
use nexuslb_scheduler::least_connections::LeastConnectionsScheduler;
use nexuslb_scheduler::least_latency::LeastLatencyScheduler;
use nexuslb_scheduler::power_of_two::PowerOfTwoChoicesScheduler;
use nexuslb_scheduler::random::RandomScheduler;
use nexuslb_scheduler::round_robin::RoundRobinScheduler;
use nexuslb_scheduler::traits::{Scheduler, SelectionContext};
use nexuslb_scheduler::weighted_round_robin::WeightedRoundRobinScheduler;
use std::net::SocketAddr;
use std::sync::Arc;

fn create_mock_backends(count: usize) -> Vec<Arc<Backend>> {
    let mut backends = Vec::with_capacity(count);
    for i in 0..count {
        let addr = format!("10.0.0.{}:8080", (i % 250) + 1)
            .parse::<SocketAddr>()
            .unwrap();
        let b = Arc::new(Backend::new(
            BackendId::new((i + 1) as u64),
            format!("backend-{}", i + 1),
            BackendAddress::new(addr),
            ((i % 5) + 1) as u32 * 20,
            Protocol::Http1,
            None,
        ));
        b.set_state(nexuslb_core::types::BackendState::Up);
        backends.push(b);
    }
    backends
}

fn bench_schedulers(c: &mut Criterion) {
    let backends = create_mock_backends(32);
    let ctx = SelectionContext::with_ip("192.168.1.100".parse().unwrap());

    let mut group = c.benchmark_group("schedulers_32_backends");

    let rr = RoundRobinScheduler::new();
    group.bench_function("round_robin", |b| {
        b.iter(|| black_box(rr.select(black_box(&backends), black_box(&ctx))))
    });

    let wrr = WeightedRoundRobinScheduler::new();
    group.bench_function("weighted_round_robin", |b| {
        b.iter(|| black_box(wrr.select(black_box(&backends), black_box(&ctx))))
    });

    let lc = LeastConnectionsScheduler::new();
    group.bench_function("least_connections", |b| {
        b.iter(|| black_box(lc.select(black_box(&backends), black_box(&ctx))))
    });

    let rand_sched = RandomScheduler::new();
    group.bench_function("random", |b| {
        b.iter(|| black_box(rand_sched.select(black_box(&backends), black_box(&ctx))))
    });

    let ip_hash = IpHashScheduler::new();
    group.bench_function("ip_hash", |b| {
        b.iter(|| black_box(ip_hash.select(black_box(&backends), black_box(&ctx))))
    });

    let ch = ConsistentHashScheduler::new();
    group.bench_function("consistent_hash", |b| {
        b.iter(|| black_box(ch.select(black_box(&backends), black_box(&ctx))))
    });

    let p2c = PowerOfTwoChoicesScheduler::new();
    group.bench_function("power_of_two_choices", |b| {
        b.iter(|| black_box(p2c.select(black_box(&backends), black_box(&ctx))))
    });

    let ll = LeastLatencyScheduler::new();
    group.bench_function("least_latency", |b| {
        b.iter(|| black_box(ll.select(black_box(&backends), black_box(&ctx))))
    });

    let ewma = EwmaLatencyScheduler::new();
    group.bench_function("ewma_latency", |b| {
        b.iter(|| black_box(ewma.select(black_box(&backends), black_box(&ctx))))
    });

    let adaptive = AdaptiveScheduler::new();
    group.bench_function("adaptive", |b| {
        b.iter(|| black_box(adaptive.select(black_box(&backends), black_box(&ctx))))
    });

    group.finish();
}

criterion_group!(benches, bench_schedulers);
criterion_main!(benches);
