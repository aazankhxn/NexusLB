use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use nexuslb_core::backend::Backend;
use nexuslb_core::types::{AlgorithmType, BackendAddress, BackendId, BackendState, Protocol};
use nexuslb_dataplane::{DataplanePipeline, DataplaneState, SharedDataplaneState};
use nexuslb_integration_tests::mock_backend::MockHttpBackend;
use nexuslb_metrics::WorkerMetrics;
use nexuslb_network::{BufferPool, ConnectionPool, ConnectionPoolConfig};
use nexuslb_proxy::rate_limiter::RateLimiter;
use nexuslb_proxy::retry::RetryPolicy;
use nexuslb_router::{PoolGroup, Router};

#[tokio::test]
async fn test_http_proxy_and_load_distribution() {
    let backend1 = MockHttpBackend::start("backend-1-response").await;
    let backend2 = MockHttpBackend::start("backend-2-response").await;

    let b1 = Arc::new(Backend::new(
        BackendId::new(1),
        "b1",
        BackendAddress::new(backend1.addr()),
        100,
        Protocol::Http1,
        None,
    ));
    b1.set_state(BackendState::Up);

    let b2 = Arc::new(Backend::new(
        BackendId::new(2),
        "b2",
        BackendAddress::new(backend2.addr()),
        100,
        Protocol::Http1,
        None,
    ));
    b2.set_state(BackendState::Up);

    let backends = vec![b1.clone(), b2.clone()];
    let pool = PoolGroup::new("default", backends, AlgorithmType::RoundRobin);
    let mut pools = HashMap::new();
    pools.insert("default".to_string(), pool);

    let router = Arc::new(Router::new(Vec::new(), pools, Some("default")));
    let rate_limiter = Arc::new(RateLimiter::new(None, None));
    let conn_pool = ConnectionPool::new(ConnectionPoolConfig::default());
    let buffer_pool = BufferPool::new(64, 4096);
    let retry_policy = RetryPolicy::default();

    let state = Arc::new(SharedDataplaneState::new(DataplaneState {
        router,
        rate_limiter,
        conn_pool,
        buffer_pool,
        retry_policy,
        tls_acceptor: None,
        http_cache: Arc::new(nexuslb_cache::HttpCache::default()),
        access_logger: Arc::new(nexuslb_observability::AccessLogger::disabled()),
        filter_chain: Arc::new(nexuslb_wasm::FilterChain::new()),
        redirect_http_to_https: false,
    }));

    let metrics = Arc::new(WorkerMetrics::new(0));

    // Spin up an in-process proxy listener
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let state_clone = state.clone();
    let metrics_clone = metrics.clone();

    tokio::spawn(async move {
        loop {
            if let Ok((stream, client_addr)) = listener.accept().await {
                let s = state_clone.clone();
                let m = metrics_clone.clone();
                tokio::spawn(async move {
                    DataplanePipeline::process_connection(stream, client_addr, s, m).await;
                });
            }
        }
    });

    // Send 10 requests through proxy
    for _ in 0..10 {
        let mut client = TcpStream::connect(proxy_addr).await.unwrap();
        client
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();

        let mut resp = Vec::new();
        client.read_to_end(&mut resp).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.contains("200 OK"));
    }

    // Verify both backends received requests
    assert!(backend1.request_count() > 0, "Backend 1 received traffic");
    assert!(backend2.request_count() > 0, "Backend 2 received traffic");
    assert_eq!(
        backend1.request_count() + backend2.request_count(),
        10,
        "All 10 requests were handled"
    );
}

#[tokio::test]
async fn test_failover_when_backend_goes_down() {
    let backend1 = MockHttpBackend::start("healthy-backend").await;

    // Backend 2 address that is offline
    let dead_addr = "127.0.0.1:49999".parse().unwrap();

    let b1 = Arc::new(Backend::new(
        BackendId::new(1),
        "b1",
        BackendAddress::new(backend1.addr()),
        100,
        Protocol::Http1,
        None,
    ));
    b1.set_state(BackendState::Up);

    let b2 = Arc::new(Backend::new(
        BackendId::new(2),
        "b2",
        BackendAddress::new(dead_addr),
        100,
        Protocol::Http1,
        None,
    ));
    // Marked down
    b2.set_state(BackendState::Down);

    let backends = vec![b1.clone(), b2.clone()];
    let pool = PoolGroup::new("default", backends, AlgorithmType::RoundRobin);
    let mut pools = HashMap::new();
    pools.insert("default".to_string(), pool);

    let router = Arc::new(Router::new(Vec::new(), pools, Some("default")));
    let rate_limiter = Arc::new(RateLimiter::new(None, None));
    let conn_pool = ConnectionPool::new(ConnectionPoolConfig::default());
    let buffer_pool = BufferPool::new(64, 4096);
    let retry_policy = RetryPolicy::default();

    let state = Arc::new(SharedDataplaneState::new(DataplaneState {
        router,
        rate_limiter,
        conn_pool,
        buffer_pool,
        retry_policy,
        tls_acceptor: None,
        http_cache: Arc::new(nexuslb_cache::HttpCache::default()),
        access_logger: Arc::new(nexuslb_observability::AccessLogger::disabled()),
        filter_chain: Arc::new(nexuslb_wasm::FilterChain::new()),
        redirect_http_to_https: false,
    }));

    let metrics = Arc::new(WorkerMetrics::new(0));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let s = state.clone();
    let m = metrics.clone();

    tokio::spawn(async move {
        loop {
            if let Ok((stream, client_addr)) = listener.accept().await {
                let s_clone = s.clone();
                let m_clone = m.clone();
                tokio::spawn(async move {
                    DataplanePipeline::process_connection(stream, client_addr, s_clone, m_clone)
                        .await;
                });
            }
        }
    });

    for _ in 0..5 {
        let mut client = TcpStream::connect(proxy_addr).await.unwrap();
        client
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();

        let mut resp = Vec::new();
        client.read_to_end(&mut resp).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.contains("healthy-backend"));
    }

    assert_eq!(backend1.request_count(), 5);
}
