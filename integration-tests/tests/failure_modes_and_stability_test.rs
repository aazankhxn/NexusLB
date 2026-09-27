use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use nexuslb_api::AdminServer;
use nexuslb_config::{AdminAuthConfig, AdminConfig, NexusConfig};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::{
    AlgorithmType, BackendAddress, BackendId, BackendState, CircuitState, Protocol,
};
use nexuslb_dataplane::{DataplanePipeline, DataplaneState, SharedDataplaneState};
use nexuslb_metrics::{GlobalMetrics, WorkerMetrics};
use nexuslb_network::{BufferPool, ConnectionPool, ConnectionPoolConfig};
use nexuslb_proxy::rate_limiter::RateLimiter;
use nexuslb_proxy::retry::RetryPolicy;
use nexuslb_router::{PoolGroup, Router};

// Helper: Setup in-process proxy listener hooked to a given backend address
async fn setup_test_proxy(
    backend_addr: std::net::SocketAddr,
    retry_policy: RetryPolicy,
) -> (
    std::net::SocketAddr,
    Arc<Backend>,
    Arc<SharedDataplaneState>,
    Arc<WorkerMetrics>,
) {
    let backend = Arc::new(Backend::new(
        BackendId::new(1),
        "test-backend-1",
        BackendAddress::new(backend_addr),
        100,
        Protocol::Http1,
        None,
    ));
    backend.set_state(BackendState::Up);

    let backends = vec![backend.clone()];
    let pool = PoolGroup::new("default", backends, AlgorithmType::RoundRobin);
    let mut pools = HashMap::new();
    pools.insert("default".to_string(), pool);

    let router = Arc::new(Router::new(Vec::new(), pools, Some("default")));
    let rate_limiter = Arc::new(RateLimiter::new(None, None));
    let conn_pool = ConnectionPool::new(ConnectionPoolConfig::default());
    let buffer_pool = BufferPool::new(32, 16384);

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

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let s_clone = state.clone();
    let m_clone = metrics.clone();

    tokio::spawn(async move {
        while let Ok((stream, client_addr)) = listener.accept().await {
            let s = s_clone.clone();
            let m = m_clone.clone();
            tokio::spawn(async move {
                DataplanePipeline::process_connection(stream, client_addr, s, m).await;
            });
        }
    });

    (proxy_addr, backend, state, metrics)
}

// -----------------------------------------------------------------------------
// TEST CASE 1: Slow Client / Incomplete Request Disconnect (Slowloris resilience)
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_case_1_slow_client_disconnect_resilience() {
    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let backend_addr = mock_listener.local_addr().unwrap();

    let (proxy_addr, _, _, _) = setup_test_proxy(backend_addr, RetryPolicy::default()).await;

    // Connect and send an incomplete fragment, then abruptly close
    let mut client = TcpStream::connect(proxy_addr).await.unwrap();
    client.write_all(b"GET /partial-re").await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(client); // Sudden client hangup

    // Ensure the proxy continues accepting subsequent connections cleanly
    let mut healthy_client = TcpStream::connect(proxy_addr).await.unwrap();
    healthy_client
        .write_all(b"GET /status HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();

    // Mock backend responds to the healthy request
    let (mut backend_stream, _) = mock_listener.accept().await.unwrap();
    let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
    backend_stream.write_all(resp).await.unwrap();

    let mut buf = vec![0u8; 512];
    let n = healthy_client.read(&mut buf).await.unwrap();
    let resp_str = String::from_utf8_lossy(&buf[..n]);
    assert!(
        resp_str.contains("200 OK"),
        "Proxy must stay healthy after slowloris disconnect"
    );
}

// -----------------------------------------------------------------------------
// TEST CASE 2: Upstream Sudden Reset / Connection Refused (Broken Upstream Failover)
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_case_2_upstream_connection_refused_error_handling() {
    // Pick an unused local port where no server is listening
    let unused_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead_backend_addr = unused_listener.local_addr().unwrap();
    drop(unused_listener); // Port is now closed and refusing connections

    let (proxy_addr, backend, _, metrics) =
        setup_test_proxy(dead_backend_addr, RetryPolicy::default()).await;

    let mut client = TcpStream::connect(proxy_addr).await.unwrap();
    client
        .write_all(b"GET /api/test HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();

    let mut buf = Vec::new();
    let _ = client.read_to_end(&mut buf).await;

    // The proxy should gracefully catch the connect failure without crashing
    assert!(
        backend.stats().total_errors() > 0,
        "Backend errors must be tracked on connection refused"
    );
    assert!(
        metrics.backend_errors_total.load(Ordering::Relaxed) > 0,
        "Worker metrics must record backend errors"
    );
}

// -----------------------------------------------------------------------------
// TEST CASE 3: Fuzzed & Malformed HTTP Inputs (Zero bytes, garbage, corrupt verbs)
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_case_3_fuzzed_malformed_http_inputs() {
    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let backend_addr = mock_listener.local_addr().unwrap();

    // Run continuous background mock server so any forwarded connection is served
    tokio::spawn(async move {
        while let Ok((mut b_stream, _)) = mock_listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = b_stream.read(&mut buf).await;
                let _ = b_stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nPONG",
                    )
                    .await;
            });
        }
    });

    let (proxy_addr, _, _, _) = setup_test_proxy(backend_addr, RetryPolicy::default()).await;

    let malformed_payloads: Vec<&[u8]> = vec![
        b"",                                                      // Empty stream
        b"\r\n\r\n",                                              // Bare newlines
        b"\xFF\xFE\x00\x01\x02\x03\x04\x05GARBAGE",               // Binary fuzz garbage
        b"GET / HTTP/9.9\r\nHost: localhost\r\n\r\n",             // Corrupt HTTP version
        b"GET / HTTP/1.1\r\nContent-Length: -500\r\n\r\n",        // Negative Content-Length
        b"GET /test\x00path HTTP/1.1\r\nHost: localhost\r\n\r\n", // Null byte in path
    ];

    for payload in malformed_payloads {
        if let Ok(mut client) = TcpStream::connect(proxy_addr).await {
            let _ = client.write_all(payload).await;
            let _ = client.shutdown().await;
            let mut resp = vec![0u8; 1024];
            let _ = tokio::time::timeout(Duration::from_millis(200), client.read(&mut resp)).await;
            // The proxy must NOT panic or crash on any fuzzed input
        }
    }

    // Verify proxy is still completely stable and functional after receiving fuzzing payloads
    let mut check_client = TcpStream::connect(proxy_addr).await.unwrap();
    check_client
        .write_all(b"GET /ping HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();

    let mut buf = vec![0u8; 512];
    let n = check_client.read(&mut buf).await.unwrap();
    let resp_str = String::from_utf8_lossy(&buf[..n]);
    assert!(
        resp_str.contains("200 OK"),
        "Proxy must survive malformed and fuzzed inputs"
    );
}

// -----------------------------------------------------------------------------
// TEST CASE 4: Circuit Breaker Cascade Isolation under Consecutive Failures
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_case_4_circuit_breaker_cascade_isolation() {
    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let backend_addr = mock_listener.local_addr().unwrap();
    drop(mock_listener); // Simulate dead backend

    let (proxy_addr, backend, state, _) =
        setup_test_proxy(backend_addr, RetryPolicy::default()).await;

    let cb_config = nexuslb_health::circuit::CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 2,
        cool_down_duration: Duration::from_millis(50),
        half_open_max_probes: 2,
    };
    let cb = nexuslb_health::circuit::CircuitBreaker::new(cb_config);

    assert_eq!(backend.circuit_state(), CircuitState::Closed);

    // Send requests that fail to connect to dead backend, propagating failures to circuit breaker
    for _ in 0..3 {
        let mut client = TcpStream::connect(proxy_addr).await.unwrap();
        let _ = client
            .write_all(b"GET /fail HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await;
        let mut buf = Vec::new();
        let _ = client.read_to_end(&mut buf).await;
        cb.on_failure(&backend);
    }

    // Circuit breaker state must now be Open to prevent cascading failure
    assert_eq!(backend.circuit_state(), CircuitState::Open);
    assert!(
        !backend.is_available(),
        "Tripped backend must be marked unavailable"
    );

    // Router selection must exclude the tripped backend
    let ctx = nexuslb_scheduler::traits::SelectionContext::default();
    let loaded = state.load();
    let pool = loaded.router.default_pool().unwrap();
    assert!(
        pool.select(&ctx).is_none(),
        "Pool selection must refuse to route to tripped backend"
    );
}

// -----------------------------------------------------------------------------
// TEST CASE 5: Oversized Request Headers Exceeding Limit (64 KB ceiling)
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_case_5_oversized_headers_rejected_with_431() {
    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let backend_addr = mock_listener.local_addr().unwrap();

    let (proxy_addr, _, _, _) = setup_test_proxy(backend_addr, RetryPolicy::default()).await;

    let mut client = TcpStream::connect(proxy_addr).await.unwrap();

    // Construct headers exceeding 64KB (60 headers * 1150 bytes = ~69 KB)
    let mut big_req = Vec::new();
    big_req.extend_from_slice(b"GET / HTTP/1.1\r\nHost: localhost\r\n");
    for i in 0..60 {
        big_req.extend_from_slice(format!("X-Header-{}: {}\r\n", i, "A".repeat(1150)).as_bytes());
    }
    big_req.extend_from_slice(b"\r\n");

    client.write_all(&big_req).await.unwrap();

    let mut resp = vec![0u8; 1024];
    let n = client.read(&mut resp).await.unwrap();
    let resp_str = String::from_utf8_lossy(&resp[..n]);

    assert!(
        resp_str.contains("431 Request Header Fields Too Large"),
        "Headers > 64KB must be rejected with 431, received: {}",
        resp_str
    );
}

// -----------------------------------------------------------------------------
// TEST CASE 6: Admin API Security Failure Modes (Auth Bypass, Tampering, Redaction)
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_case_6_admin_api_security_failure_modes() {
    let initial_state = nexuslb_dataplane::DataplaneState {
        router: Arc::new(Router::new(Vec::new(), HashMap::new(), None)),
        rate_limiter: Arc::new(RateLimiter::new(None, None)),
        conn_pool: ConnectionPool::new(ConnectionPoolConfig::default()),
        buffer_pool: BufferPool::new(16, 4096),
        retry_policy: RetryPolicy::default(),
        tls_acceptor: None,
        http_cache: Arc::new(nexuslb_cache::HttpCache::default()),
        access_logger: Arc::new(nexuslb_observability::AccessLogger::disabled()),
        filter_chain: Arc::new(nexuslb_wasm::FilterChain::new()),
        redirect_http_to_https: false,
    };
    let shared_state = Arc::new(SharedDataplaneState::new(initial_state));
    let metrics = Arc::new(GlobalMetrics::new(1));

    let config = Arc::new(NexusConfig {
        server: nexuslb_config::ServerConfig {
            listen: vec!["127.0.0.1:8080".to_string()],
            workers: "1".to_string(),
            engine: "tokio".to_string(),
            reuse_port: false,
            tcp_nodelay: true,
            max_connections: None,
        },
        load_balancer: Default::default(),
        backends: vec![],
        routes: vec![],
        health_check: Default::default(),
        circuit_breaker: Default::default(),
        tls: Default::default(),
        metrics: Default::default(),
        admin: AdminConfig {
            enabled: true,
            address: "127.0.0.1:0".to_string(),
            authentication: AdminAuthConfig {
                required: true,
                allow_unauthenticated_health: true,
            },
            token: Some("secure-admin-token-12345".to_string()),
            mutation_token: Some("super-mutation-token-99999".to_string()),
        },
        rate_limit: Default::default(),
        access_log: Default::default(),
        discovery: Default::default(),
        limits: Default::default(),
    });

    let admin_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let admin_addr = admin_listener.local_addr().unwrap();
    drop(admin_listener);

    let reloader: nexuslb_api::ReloadHandler = Arc::new(|| Ok("Reloaded".to_string()));
    let admin = AdminServer::new(
        admin_addr,
        Some("secure-admin-token-12345".to_string()),
        metrics,
        shared_state,
        config,
    )
    .with_reloader(reloader);

    tokio::spawn(async move {
        let _ = admin.run().await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Subtest A: Unauthenticated request to /metrics must be rejected with 401
    {
        let mut stream = TcpStream::connect(admin_addr).await.unwrap();
        stream
            .write_all(b"GET /metrics HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        let resp = String::from_utf8_lossy(&buf);
        assert!(
            resp.contains("401 Unauthorized"),
            "Protected endpoint must reject unauthenticated request"
        );
    }

    // Subtest B: Tampered / Wrong token must be rejected with 401
    {
        let mut stream = TcpStream::connect(admin_addr).await.unwrap();
        stream.write_all(b"GET /metrics HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer wrong-token-xyz\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        let resp = String::from_utf8_lossy(&buf);
        assert!(
            resp.contains("401 Unauthorized"),
            "Wrong token must be rejected with 401"
        );
    }

    // Subtest C: Read token attempting a mutation endpoint (POST /reload) must be rejected with 403 Forbidden
    {
        let mut stream = TcpStream::connect(admin_addr).await.unwrap();
        let req = "POST /reload HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer secure-admin-token-12345\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        let resp = String::from_utf8_lossy(&buf);
        assert!(
            resp.contains("403 Forbidden"),
            "Read token must be denied mutation privileges (RBAC)"
        );
    }

    // Subtest D: Valid mutation token succeeds with 200 OK
    {
        let mut stream = TcpStream::connect(admin_addr).await.unwrap();
        let req = "POST /reload HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer super-mutation-token-99999\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        let resp = String::from_utf8_lossy(&buf);
        assert!(resp.contains("200 OK"), "Valid mutation token must succeed");
    }

    // Subtest E: GET /config must redact secret tokens
    {
        let mut stream = TcpStream::connect(admin_addr).await.unwrap();
        let req = "GET /config HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer secure-admin-token-12345\r\nConnection: close\r\n\r\n";
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        let resp = String::from_utf8_lossy(&buf);
        assert!(resp.contains("200 OK"));
        assert!(
            resp.contains("[REDACTED]"),
            "Config API must redact secrets"
        );
        assert!(
            !resp.contains("secure-admin-token-12345"),
            "Config API must NEVER expose cleartext admin tokens"
        );
    }
}

// -----------------------------------------------------------------------------
// TEST CASE 7: Pipelined Keep-Alive Stress with Alternating Payloads
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_case_7_pipelined_keepalive_stability() {
    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let backend_addr = mock_listener.local_addr().unwrap();

    let (proxy_addr, _, _, _) = setup_test_proxy(backend_addr, RetryPolicy::default()).await;

    // Background mock backend handling 3 keep-alive requests on the same connection
    tokio::spawn(async move {
        if let Ok((mut b_stream, _)) = mock_listener.accept().await {
            for i in 1..=3 {
                let mut req_buf = vec![0u8; 1024];
                let n = b_stream.read(&mut req_buf).await.unwrap();
                if n == 0 {
                    break;
                }
                let body = format!("Pipeline-Response-{}", i);
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n{}",
                    body.len(),
                    body
                );
                b_stream.write_all(resp.as_bytes()).await.unwrap();
            }
        }
    });

    let mut client = TcpStream::connect(proxy_addr).await.unwrap();

    // Send 3 sequential requests over the single persistent keep-alive TCP stream
    for i in 1..=3 {
        let req = format!(
            "POST /pipeline/{} HTTP/1.1\r\nHost: localhost\r\nContent-Length: 4\r\nConnection: keep-alive\r\n\r\nDATA",
            i
        );
        client.write_all(req.as_bytes()).await.unwrap();

        let mut resp_buf = vec![0u8; 512];
        let n = client.read(&mut resp_buf).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp_buf[..n]);
        assert!(
            resp_str.contains("200 OK"),
            "Pipelined request {} must return 200 OK",
            i
        );
        assert!(resp_str.contains(&format!("Pipeline-Response-{}", i)));
    }
}
