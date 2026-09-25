use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::{AlgorithmType, BackendAddress, BackendId, Protocol};
use nexuslb_router::{HostMatch, PathMatch, PoolGroup, Route, Router};
use std::collections::HashMap;
use std::sync::Arc;

fn build_bench_router() -> Router {
    let mut backends = Vec::new();
    for i in 1..=4 {
        let b = Arc::new(Backend::new(
            BackendId::new(i),
            format!("api-{}", i),
            BackendAddress::new(format!("127.0.0.1:{}", 8080 + i).parse().unwrap()),
            100,
            Protocol::Http1,
            None,
        ));
        b.set_state(nexuslb_core::types::BackendState::Up);
        backends.push(b);
    }

    let mut pools = HashMap::new();
    pools.insert(
        "api-pool".to_string(),
        PoolGroup::new("api-pool", backends.clone(), AlgorithmType::Adaptive),
    );
    pools.insert(
        "static-pool".to_string(),
        PoolGroup::new("static-pool", backends.clone(), AlgorithmType::RoundRobin),
    );

    let routes = vec![
        Route {
            name: "api-v1".to_string(),
            host: HostMatch::Exact("api.example.com".to_string()),
            path: PathMatch::Prefix("/api/v1/".to_string()),
            methods: Some(vec!["GET".to_string(), "POST".to_string()]),
            headers: None,
            sni: None,
            pool_name: "api-pool".to_string(),
            priority: 100,
        },
        Route {
            name: "static-assets".to_string(),
            host: HostMatch::Any,
            path: PathMatch::Prefix("/static/".to_string()),
            methods: None,
            headers: None,
            sni: None,
            pool_name: "static-pool".to_string(),
            priority: 50,
        },
    ];

    Router::new(routes, pools, Some("api-pool"))
}

fn bench_router_match(c: &mut Criterion) {
    let router = build_bench_router();

    c.bench_function("router_prefix_match", |b| {
        b.iter(|| {
            black_box(router.route(
                black_box(Some("api.example.com")),
                black_box("/api/v1/users/profile"),
                black_box(Some("GET")),
                black_box(None),
                black_box(None),
            ))
        })
    });
}

criterion_group!(benches, bench_router_match);
criterion_main!(benches);
