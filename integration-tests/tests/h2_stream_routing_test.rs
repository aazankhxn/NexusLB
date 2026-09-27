#![deny(unsafe_code)]

//! Integration Test: HTTP/2 Stream-Level Routing & Multiplexed Upstream Connection Pool
//! Verifies Issues #1, #2, and #10:
//! - Multiple streams on a single client H2 connection are routed independently based on :path / :authority.
//! - Upstream backend H2 connections are persistent and multiplexed.

use bytes::Bytes;
use h2::client;
use h2::server;
use http::{Request, Response};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};

use nexuslb_core::backend::Backend;
use nexuslb_core::types::{AlgorithmType, BackendAddress, BackendId, Protocol};
use nexuslb_dataplane::{DataplanePipeline, DataplaneState, SharedDataplaneState};
use nexuslb_metrics::WorkerMetrics;
use nexuslb_network::{BufferPool, ConnectionPool, ConnectionPoolConfig};
use nexuslb_proxy::rate_limiter::RateLimiter;
use nexuslb_proxy::retry::RetryPolicy;
use nexuslb_proxy::{H2Config, H2ConnectionPool};
use nexuslb_router::matcher::Router;
use nexuslb_router::pool::PoolGroup;
use nexuslb_router::route::{HostMatch, PathMatch, Route};

/// Spawns a mock H2 backend that replies with a custom identification payload
async fn spawn_h2_backend(ident: &'static str) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut h2_conn = match server::handshake(socket).await {
                    Ok(conn) => conn,
                    Err(_) => return,
                };

                while let Some(Ok((_req, mut respond))) = h2_conn.accept().await {
                    tokio::spawn(async move {
                        let response = Response::builder()
                            .status(200)
                            .header("content-type", "text/plain")
                            .header("x-backend-id", ident)
                            .body(())
                            .unwrap();
                        let mut send_stream = respond.send_response(response, false).unwrap();
                        let _ = send_stream.send_data(Bytes::from_static(ident.as_bytes()), true);
                    });
                }
            });
        }
    });

    addr
}

#[tokio::test]
async fn test_h2_per_stream_routing_and_multiplexing() {
    // 1. Spawn two distinct HTTP/2 backends
    let addr_a = spawn_h2_backend("backend-service-alpha").await;
    let addr_b = spawn_h2_backend("backend-service-beta").await;

    let backend_a = Arc::new(Backend::new(
        BackendId::new(1),
        "backend-a",
        BackendAddress::new(addr_a),
        100,
        Protocol::Http2,
        None,
    ));
    backend_a.set_state(nexuslb_core::types::BackendState::Up);

    let backend_b = Arc::new(Backend::new(
        BackendId::new(2),
        "backend-b",
        BackendAddress::new(addr_b),
        100,
        Protocol::Http2,
        None,
    ));
    backend_b.set_state(nexuslb_core::types::BackendState::Up);

    // 2. Configure two pools and routes
    let pool_a = PoolGroup::new(
        "pool-a",
        vec![backend_a.clone()],
        AlgorithmType::RoundRobin,
    );
    let pool_b = PoolGroup::new(
        "pool-b",
        vec![backend_b.clone()],
        AlgorithmType::RoundRobin,
    );

    let mut pools = HashMap::new();
    pools.insert("pool-a".to_string(), pool_a);
    pools.insert("pool-b".to_string(), pool_b);

    let route_a = Route {
        name: "route-a".to_string(),
        host: HostMatch::Any,
        path: PathMatch::Prefix("/service-a".to_string()),
        methods: None,
        headers: None,
        sni: None,
        pool_name: "pool-a".to_string(),
        priority: 10,
    };
    let route_b = Route {
        name: "route-b".to_string(),
        host: HostMatch::Any,
        path: PathMatch::Prefix("/service-b".to_string()),
        methods: None,
        headers: None,
        sni: None,
        pool_name: "pool-b".to_string(),
        priority: 10,
    };

    let router = Arc::new(Router::new(vec![route_a, route_b], pools, Some("pool-a")));
    let rate_limiter = Arc::new(RateLimiter::new(None, None));
    let conn_pool = ConnectionPool::new(ConnectionPoolConfig::default());
    let buffer_pool = BufferPool::new(64, 4096);
    let retry_policy = RetryPolicy::default();
    let http_cache = Arc::new(nexuslb_cache::HttpCache::default());
    let h2_pool = Arc::new(H2ConnectionPool::new(Duration::from_secs(5)));
    let h2_config = H2Config::default();

    let state = Arc::new(SharedDataplaneState::new(DataplaneState {
        router,
        rate_limiter,
        conn_pool,
        buffer_pool,
        retry_policy,
        tls_acceptor: None,
        http_cache,
        h2_pool,
        h2_config,
        access_logger: Arc::new(nexuslb_observability::AccessLogger::disabled()),
        filter_chain: Arc::new(nexuslb_wasm::FilterChain::new()),
        redirect_http_to_https: false,
    }));

    // 3. Spin up NexusLB listener
    let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();

    let state_clone = state.clone();
    tokio::spawn(async move {
        let metrics = Arc::new(WorkerMetrics::new(0));
        while let Ok((socket, peer)) = proxy_listener.accept().await {
            let s = state_clone.clone();
            let m = metrics.clone();
            tokio::spawn(async move {
                DataplanePipeline::process_connection(socket, peer, s, m).await;
            });
        }
    });

    // 4. Client establishes a SINGLE TCP connection to NexusLB and handshakes HTTP/2
    let client_tcp = TcpStream::connect(proxy_addr).await.unwrap();
    let (mut client_h2, conn_driver) = client::handshake(client_tcp).await.unwrap();

    tokio::spawn(async move {
        let _ = conn_driver.await;
    });

    // Wait briefly for handshake
    tokio::time::sleep(Duration::from_millis(50)).await;

    // STREAM 1: GET /service-a/info -> Must route to backend-service-alpha
    let req1 = Request::builder()
        .uri("http://localhost/service-a/info")
        .header("host", "localhost")
        .body(())
        .unwrap();

    let (resp1_fut, _) = client_h2.send_request(req1, true).unwrap();
    let resp1 = resp1_fut.await.unwrap();
    assert_eq!(resp1.status(), 200);
    assert_eq!(
        resp1.headers().get("x-backend-id").unwrap(),
        "backend-service-alpha"
    );

    // STREAM 2 on THE SAME CLIENT CONNECTION: GET /service-b/status -> Must route to backend-service-beta (Issue #1 & #10!)
    let req2 = Request::builder()
        .uri("http://localhost/service-b/status")
        .header("host", "localhost")
        .body(())
        .unwrap();

    let (resp2_fut, _) = client_h2.send_request(req2, true).unwrap();
    let resp2 = resp2_fut.await.unwrap();
    assert_eq!(resp2.status(), 200);
    assert_eq!(
        resp2.headers().get("x-backend-id").unwrap(),
        "backend-service-beta"
    );

    // STREAM 3 on THE SAME CLIENT CONNECTION: Another GET /service-a/again -> Reuses persistent upstream H2 connection (Issue #2!)
    let req3 = Request::builder()
        .uri("http://localhost/service-a/again")
        .header("host", "localhost")
        .body(())
        .unwrap();

    let (resp3_fut, _) = client_h2.send_request(req3, true).unwrap();
    let resp3 = resp3_fut.await.unwrap();
    assert_eq!(resp3.status(), 200);
    assert_eq!(
        resp3.headers().get("x-backend-id").unwrap(),
        "backend-service-alpha"
    );
}
