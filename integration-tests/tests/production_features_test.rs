use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use nexuslb_api::AdminServer;
use nexuslb_config::NexusConfig;
use nexuslb_core::backend::Backend;
use nexuslb_core::types::{AlgorithmType, BackendAddress, BackendId, BackendState, Protocol};
use nexuslb_dataplane::{DataplanePipeline, DataplaneState, SharedDataplaneState};
use nexuslb_discovery::{DiscoveryManager, FileCatalogDiscovery};
use nexuslb_metrics::{GlobalMetrics, WorkerMetrics};
use nexuslb_network::{BufferPool, ConnectionPool, ConnectionPoolConfig};
use nexuslb_observability::{AccessLogEntry, AccessLogger};
use nexuslb_proxy::rate_limiter::RateLimiter;
use nexuslb_proxy::retry::RetryPolicy;
use nexuslb_router::{PoolGroup, Router};
use nexuslb_wasm::{FilterChain, HeaderRewriteFilter, JwtAuthFilter};

fn create_test_state(pool_name: &str, redirect_http: bool) -> DataplaneState {
    let mut pools = HashMap::new();
    let backend = Arc::new(Backend::new(
        BackendId::new(1),
        "test-backend",
        BackendAddress::new("127.0.0.1:9999".parse().unwrap()),
        100,
        Protocol::Http1,
        None,
    ));
    backend.set_state(BackendState::Up);
    let pool = PoolGroup::new(pool_name, vec![backend], AlgorithmType::RoundRobin);
    pools.insert(pool_name.to_string(), pool);

    let router = Arc::new(Router::new(Vec::new(), pools, Some(pool_name)));
    let rate_limiter = Arc::new(RateLimiter::new(None, None));
    let conn_pool = ConnectionPool::new(ConnectionPoolConfig::default());
    let buffer_pool = BufferPool::new(16, 4096);
    let retry_policy = RetryPolicy::default();
    let http_cache = Arc::new(nexuslb_cache::HttpCache::default());
    let (access_logger, _) = AccessLogger::new(false, "json", "stdout");

    DataplaneState {
        router,
        rate_limiter,
        conn_pool,
        buffer_pool,
        retry_policy,
        tls_acceptor: None,
        http_cache,
        access_logger: Arc::new(access_logger),
        filter_chain: Arc::new(FilterChain::new()),
        redirect_http_to_https: redirect_http,
    }
}

#[tokio::test]
async fn test_hot_reload_admin_api() {
    let initial_state = create_test_state("initial-pool", false);
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
        admin: Default::default(),
        rate_limit: Default::default(),
        access_log: Default::default(),
        discovery: Default::default(),
    });

    let reload_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = reload_counter.clone();
    let state_clone = shared_state.clone();

    let reloader: nexuslb_api::ReloadHandler = Arc::new(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        let new_state = create_test_state("reloaded-pool", false);
        state_clone.swap(new_state);
        Ok("Reloaded successfully".to_string())
    });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let admin_addr = listener.local_addr().unwrap();
    drop(listener);

    let admin = AdminServer::new(admin_addr, None, metrics, shared_state.clone(), config)
        .with_reloader(reloader);

    tokio::spawn(async move {
        let _ = admin.run().await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Send POST /reload request
    let mut stream = TcpStream::connect(admin_addr).await.unwrap();
    let req = format!(
        "POST /reload HTTP/1.1\r\nHost: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        admin_addr
    );
    stream.write_all(req.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    let resp = String::from_utf8_lossy(&buf);

    assert!(resp.contains("200 OK"));
    assert!(resp.contains("Reloaded successfully"));
    assert_eq!(reload_counter.load(Ordering::SeqCst), 1);
    assert_eq!(
        shared_state
            .load()
            .router
            .default_pool()
            .map(|p| p.name.as_str()),
        Some("reloaded-pool")
    );
}

#[tokio::test]
async fn test_jwt_and_rewrite_filter_pipeline() {
    let mut chain = FilterChain::new();
    chain.add_filter(Arc::new(JwtAuthFilter::new("/protected")));

    let rewrite = HeaderRewriteFilter::new()
        .with_request_header("X-Injected-By", "NexusLB-Proxy")
        .with_remove_header("X-Remove-Me");
    chain.add_filter(Arc::new(rewrite));

    // Case 1: Unprotected path passes through, header rewrite applies
    let mut method = "GET".to_string();
    let mut path = "/public/index.html".to_string();
    let mut headers = vec![
        ("X-Remove-Me".to_string(), "sensitive-data".to_string()),
        ("User-Agent".to_string(), "curl/7.68".to_string()),
    ];

    let action = chain.execute_request(&mut method, &mut path, &mut headers);
    match action {
        nexuslb_wasm::FilterAction::Continue => {
            assert!(!headers.iter().any(|(k, _)| k == "X-Remove-Me"));
            assert!(headers
                .iter()
                .any(|(k, v)| k == "X-Injected-By" && v == "NexusLB-Proxy"));
        }
        _ => panic!("Expected Continue for public path"),
    }

    // Case 2: Protected path without token returns 401
    let mut method = "POST".to_string();
    let mut path = "/protected/user/profile".to_string();
    let mut headers = vec![];
    let action = chain.execute_request(&mut method, &mut path, &mut headers);
    match action {
        nexuslb_wasm::FilterAction::StopAndReply { status, .. } => {
            assert_eq!(status, 401);
        }
        _ => panic!("Expected 401 StopAndReply for unauthenticated protected request"),
    }

    // Case 3: Protected path with valid JWT token passes through
    let mut method = "GET".to_string();
    let mut path = "/protected/dashboard".to_string();
    let mut headers = vec![(
        "Authorization".to_string(),
        "Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozGzV".to_string(),
    )];
    let action = chain.execute_request(&mut method, &mut path, &mut headers);
    assert_eq!(action, nexuslb_wasm::FilterAction::Continue);
    assert!(headers
        .iter()
        .any(|(k, v)| k == "X-User-Id" && v == "authenticated-user"));
    assert!(headers
        .iter()
        .any(|(k, v)| k == "X-Injected-By" && v == "NexusLB-Proxy"));
}

#[tokio::test]
async fn test_access_logger_formats() {
    let client_ip = "192.168.1.100".parse().unwrap();
    let entry = AccessLogEntry::new(
        client_ip,
        "GET",
        "/api/v1/status",
        200,
        Duration::from_micros(1250),
        "backend-prod-01",
        512,
    );

    // Verify JSON format
    let json_line = entry.format_json();
    assert!(json_line.contains("\"client_ip\":\"192.168.1.100\""));
    assert!(json_line.contains("\"method\":\"GET\""));
    assert!(json_line.contains("\"path\":\"/api/v1/status\""));
    assert!(json_line.contains("\"status\":200"));
    assert!(json_line.contains("\"backend\":\"backend-prod-01\""));
    assert!(json_line.contains("\"bytes_sent\":512"));

    // Verify Combined format
    let combined_line = entry.format_combined();
    assert!(combined_line.starts_with("192.168.1.100 - - ["));
    assert!(combined_line
        .contains("\"GET /api/v1/status HTTP/1.1\" 200 512 \"backend-prod-01\" 1.25ms"));

    // Test non-blocking background logger dispatch
    let (logger, _) = AccessLogger::new(true, "json", "stdout");
    logger.log(entry);
}

#[tokio::test]
async fn test_http_to_https_301_redirect() {
    let state = create_test_state("default", true); // redirect_http_to_https: true
    let shared_state = Arc::new(SharedDataplaneState::new(state));
    let metrics = Arc::new(WorkerMetrics::new(0));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let state_clone = shared_state.clone();
    let metrics_clone = metrics.clone();

    tokio::spawn(async move {
        if let Ok((stream, client_addr)) = listener.accept().await {
            DataplanePipeline::process_connection(stream, client_addr, state_clone, metrics_clone)
                .await;
        }
    });

    let mut stream = TcpStream::connect(proxy_addr).await.unwrap();
    let req = "GET /login?ref=web HTTP/1.1\r\nHost: mydomain.org:8080\r\nConnection: close\r\n\r\n";
    stream.write_all(req.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    let resp = String::from_utf8_lossy(&buf);

    assert!(resp.contains("HTTP/1.1 301 Moved Permanently"));
    assert!(resp.contains("Location: https://mydomain.org:8080/login?ref=web"));
}

#[tokio::test]
async fn test_service_discovery_file_provider() {
    let json_data = r#"[
        {
            "name": "srv-1",
            "address": "127.0.0.1:8001",
            "weight": 100,
            "pool": "web"
        },
        {
            "name": "srv-2",
            "address": "127.0.0.1:8002",
            "weight": 50,
            "pool": "web"
        }
    ]"#;

    let tmp_path = std::env::temp_dir().join(format!("nexuslb_disc_{}.json", std::process::id()));
    tokio::fs::write(&tmp_path, json_data).await.unwrap();

    let mut manager = DiscoveryManager::new();
    manager.add_provider(Box::new(FileCatalogDiscovery::new(&tmp_path)));

    let instances = manager.sync_once().await;
    let _ = tokio::fs::remove_file(tmp_path).await;

    assert_eq!(instances.len(), 2);
    assert_eq!(instances[0].name, "srv-1");
    assert_eq!(instances[0].address, "127.0.0.1:8001".parse().unwrap());
    assert_eq!(instances[0].weight, 100);
    assert_eq!(instances[1].name, "srv-2");
    assert_eq!(instances[1].address, "127.0.0.1:8002".parse().unwrap());
    assert_eq!(instances[1].weight, 50);
}

#[derive(Debug)]
struct DangerAcceptAnyCert;

impl rustls::client::danger::ServerCertVerifier for DangerAcceptAnyCert {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls_pki_types::CertificateDer<'_>,
        _intermediates: &[rustls_pki_types::CertificateDer<'_>],
        _server_name: &rustls_pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls_pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls_pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls_pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

#[tokio::test]
async fn test_tls_termination_and_sni_resolution() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    // 1. Spin up a mock backend HTTP server
    let backend_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let backend_addr = backend_listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = backend_listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                let body = "Hello from TLS-terminated backend!";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            });
        }
    });

    // 2. Generate self-signed TLS certificates
    let (cert_pem, key_pem) =
        nexuslb_tls::generate_self_signed(vec!["localhost".to_string(), "127.0.0.1".to_string()])
            .unwrap();
    let certs = nexuslb_tls::parse_certs_from_pem(&cert_pem).unwrap();
    let key = nexuslb_tls::parse_key_from_pem(&key_pem).unwrap();

    let sni_resolver = Arc::new(nexuslb_tls::DynamicSniResolver::new());
    sni_resolver.set_default_certificate(certs, key).unwrap();

    let tls_acceptor = nexuslb_tls::TlsConfigBuilder::build_acceptor(sni_resolver).unwrap();

    // 3. Set up load balancer state pointing to the backend
    let mut pools = HashMap::new();
    let backend = Arc::new(Backend::new(
        BackendId::new(10),
        "tls-backend",
        BackendAddress::new(backend_addr),
        100,
        Protocol::Http1,
        None,
    ));
    backend.set_state(BackendState::Up);
    let pool = PoolGroup::new("default", vec![backend], AlgorithmType::RoundRobin);
    pools.insert("default".to_string(), pool);

    let router = Arc::new(Router::new(Vec::new(), pools, Some("default")));
    let rate_limiter = Arc::new(RateLimiter::new(None, None));
    let conn_pool = ConnectionPool::new(ConnectionPoolConfig::default());
    let buffer_pool = BufferPool::new(16, 4096);
    let retry_policy = RetryPolicy::default();
    let http_cache = Arc::new(nexuslb_cache::HttpCache::default());
    let (access_logger, _) = AccessLogger::new(false, "json", "stdout");

    let state = Arc::new(SharedDataplaneState::new(DataplaneState {
        router,
        rate_limiter,
        conn_pool,
        buffer_pool,
        retry_policy,
        tls_acceptor: Some(tls_acceptor),
        http_cache,
        access_logger: Arc::new(access_logger),
        filter_chain: Arc::new(FilterChain::new()),
        redirect_http_to_https: false,
    }));

    let metrics = Arc::new(WorkerMetrics::new(0));

    // 4. Spin up Load Balancer proxy listener
    let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();

    let state_clone = state.clone();
    let metrics_clone = metrics.clone();

    tokio::spawn(async move {
        while let Ok((stream, client_addr)) = proxy_listener.accept().await {
            let s = state_clone.clone();
            let m = metrics_clone.clone();
            tokio::spawn(async move {
                DataplanePipeline::process_connection(stream, client_addr, s, m).await;
            });
        }
    });

    // 5. Connect via TLS client
    let mut client_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(DangerAcceptAnyCert))
        .with_no_client_auth();
    client_config.alpn_protocols = vec![b"http/1.1".to_vec()];

    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
    let tcp_stream = TcpStream::connect(proxy_addr).await.unwrap();
    let server_name = rustls_pki_types::ServerName::try_from("localhost")
        .unwrap()
        .to_owned();

    let mut tls_stream = connector.connect(server_name, tcp_stream).await.unwrap();

    // 6. Send HTTP GET over TLS
    let req = "GET /test-tls HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    tls_stream.write_all(req.as_bytes()).await.unwrap();
    tls_stream.flush().await.unwrap();

    // 7. Receive and verify response
    let mut resp_buf = [0u8; 4096];
    let n = tls_stream.read(&mut resp_buf).await.unwrap();
    let resp = String::from_utf8_lossy(&resp_buf[..n]);

    assert!(resp.contains("HTTP/1.1 200 OK"));
    assert!(resp.contains("Hello from TLS-terminated backend!"));
}
