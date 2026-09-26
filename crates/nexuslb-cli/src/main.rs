#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{error, info};

use nexuslb_api::AdminServer;
use nexuslb_config::{load_from_file, NexusConfig};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::{AlgorithmType, BackendAddress, BackendId, BackendState, Protocol};
use nexuslb_dataplane::{DataplaneState, SharedDataplaneState};
use nexuslb_engine_io_uring::IoUringEngine;
use nexuslb_engine_tokio::TokioEngine;
use nexuslb_engine_xdp::XdpEngine;
use nexuslb_health::{ActiveHealthCheckConfig, ActiveHealthChecker, HealthCheckType};
use nexuslb_metrics::GlobalMetrics;
use nexuslb_network::{BufferPool, ConnectionPool, ConnectionPoolConfig, SocketConfig};
use nexuslb_observability::{init_observability, AccessLogger};
use nexuslb_proxy::rate_limiter::RateLimiter;
use nexuslb_proxy::retry::RetryPolicy;
use nexuslb_router::{HostMatch, PathMatch, PoolGroup, Route, Router};
use nexuslb_tls::{DynamicSniResolver, TlsConfigBuilder};
use nexuslb_wasm::{FilterChain, HeaderRewriteFilter, JwtAuthFilter};

#[derive(Parser)]
#[command(name = "nexuslb")]
#[command(about = "NexusLB — High-performance adaptive load balancing for modern infrastructure.")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the NexusLB load balancer
    Start {
        #[arg(short, long, default_value = "nexuslb.yaml")]
        config: String,

        #[arg(short, long)]
        engine: Option<String>,

        #[arg(short, long)]
        workers: Option<String>,

        #[arg(long, default_value = "info")]
        log_level: String,

        #[arg(long)]
        json_logs: bool,
    },
    /// Validate configuration file without starting
    Check {
        #[arg(short, long, default_value = "nexuslb.yaml")]
        config: String,
    },
    /// Reload configuration on a running NexusLB instance
    Reload {
        #[arg(short, long, default_value = "127.0.0.1:9091")]
        admin_addr: String,

        #[arg(short, long)]
        token: Option<String>,
    },
    /// Query status of a running NexusLB instance
    Status {
        #[arg(short, long, default_value = "127.0.0.1:9091")]
        admin_addr: String,
    },
    /// Run load tests or collect system benchmark information
    Benchmark {
        #[command(subcommand)]
        bench_command: BenchCommands,
    },
    /// Launch interactive real-time terminal dashboard (TUI)
    Top {
        #[arg(short, long, default_value = "http://127.0.0.1:9091")]
        admin_url: String,
    },
    /// Print version and compilation information
    Version,
}

#[derive(Subcommand)]
enum BenchCommands {
    /// Collect system information for reproducible benchmarks
    SystemInfo,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => {
            println!("NexusLB v{}", env!("CARGO_PKG_VERSION"));
            println!(
                "Target:        {} ({})",
                std::env::consts::OS,
                std::env::consts::ARCH
            );
            println!("Engines:       tokio, io-uring (Linux), xdp (research)");
            println!("Optimizations: LTO=fat, codegen-units=1, panic=abort");
        }
        Commands::Benchmark { bench_command } => match bench_command {
            BenchCommands::SystemInfo => {
                print_system_info();
            }
        },
        Commands::Check { config } => match load_from_file(&config) {
            Ok(_) => {
                println!("Configuration '{}' is valid.", config);
            }
            Err(e) => {
                eprintln!("Configuration validation failed for '{}': {}", config, e);
                std::process::exit(1);
            }
        },
        Commands::Reload { admin_addr, token } => {
            println!("Sending reload request to NexusLB at {}...", admin_addr);
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let client = rt.block_on(req_client_post_helper(&admin_addr, "/reload", token));
            match client {
                Ok(resp) => println!("Reload response: {}", resp),
                Err(e) => eprintln!("Failed to signal reload: {}", e),
            }
        }
        Commands::Status { admin_addr } => {
            println!("Querying NexusLB status at {}...", admin_addr);
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let client = rt.block_on(req_client_helper(&admin_addr, "/backends", None));
            match client {
                Ok(resp) => println!("Active Backends:\n{}", resp),
                Err(e) => eprintln!("Failed to retrieve status: {}", e),
            }
        }
        Commands::Top { admin_url } => {
            let url = if !admin_url.starts_with("http://") && !admin_url.starts_with("https://") {
                format!("http://{}", admin_url)
            } else {
                admin_url
            };
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(nexuslb_tui::run_dashboard(url))?;
        }
        Commands::Start {
            config,
            engine,
            workers,
            log_level,
            json_logs,
        } => {
            init_observability(&log_level, json_logs);

            info!(
                version = env!("CARGO_PKG_VERSION"),
                config_path = %config,
                "Starting NexusLB"
            );

            let cfg = match load_from_file(&config) {
                Ok(c) => Arc::new(c),
                Err(e) => {
                    error!(error = %e, "Configuration error");
                    std::process::exit(1);
                }
            };

            // Determine workers and engine override
            let workers_spec = workers.unwrap_or_else(|| cfg.server.workers.clone());
            let engine_spec = engine.unwrap_or_else(|| cfg.server.engine.clone());

            let num_workers = match workers_spec.trim().to_ascii_lowercase().as_str() {
                "auto" => std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4),
                other => other.parse::<usize>().unwrap_or(4),
            };

            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(num_workers)
                .enable_all()
                .build()?;

            rt.block_on(async { start_nexuslb(cfg, &config, &engine_spec, &workers_spec).await })?;
        }
    }

    Ok(())
}

fn print_system_info() {
    println!("=== NexusLB Benchmark System Information ===");
    println!("OS:             {}", std::env::consts::OS);
    println!("Architecture:   {}", std::env::consts::ARCH);
    println!(
        "Logical Cores:  {}",
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    );
    println!("Rust Version:   {}", env!("CARGO_PKG_VERSION"));
    println!("Profile:        Release");
    println!("============================================");
}

async fn req_client_helper(
    admin_addr: &str,
    endpoint: &str,
    token: Option<String>,
) -> anyhow::Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let addr: SocketAddr = admin_addr.parse()?;
    let mut stream = TcpStream::connect(addr).await?;

    let auth_header = if let Some(t) = token {
        format!("Authorization: Bearer {}\r\n", t)
    } else {
        String::new()
    };

    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\n{}Connection: close\r\n\r\n",
        endpoint, admin_addr, auth_header
    );
    stream.write_all(req.as_bytes()).await?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    let resp = String::from_utf8_lossy(&buf);

    if let Some(idx) = resp.find("\r\n\r\n") {
        Ok(resp[idx + 4..].to_string())
    } else {
        Ok(resp.to_string())
    }
}

async fn req_client_post_helper(
    admin_addr: &str,
    endpoint: &str,
    token: Option<String>,
) -> anyhow::Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let addr: SocketAddr = admin_addr.parse()?;
    let mut stream = TcpStream::connect(addr).await?;

    let auth_header = if let Some(t) = token {
        format!("Authorization: Bearer {}\r\n", t)
    } else {
        String::new()
    };

    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\n{}Content-Length: 0\r\nConnection: close\r\n\r\n",
        endpoint, admin_addr, auth_header
    );
    stream.write_all(req.as_bytes()).await?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    let resp = String::from_utf8_lossy(&buf);

    if let Some(idx) = resp.find("\r\n\r\n") {
        Ok(resp[idx + 4..].to_string())
    } else {
        Ok(resp.to_string())
    }
}

fn build_dataplane_state(cfg: &NexusConfig) -> anyhow::Result<(DataplaneState, Vec<Arc<Backend>>)> {
    // 1. Build backends and pools
    let mut backends = Vec::new();
    let mut pool_backend_map: HashMap<String, Vec<Arc<Backend>>> = HashMap::new();

    for (i, b_cfg) in cfg.backends.iter().enumerate() {
        let addr: SocketAddr = b_cfg.address.parse()?;
        let proto = match b_cfg.protocol.to_ascii_lowercase().as_str() {
            "tcp" => Protocol::Tcp,
            "http2" => Protocol::Http2,
            _ => Protocol::Http1,
        };

        let backend = Arc::new(Backend::new(
            BackendId::new((i + 1) as u64),
            &b_cfg.name,
            BackendAddress::new(addr),
            b_cfg.weight,
            proto,
            b_cfg.max_connections,
        ));

        // Initial state UP
        backend.set_state(BackendState::Up);
        backends.push(backend.clone());

        let pool_name = b_cfg.pool.clone().unwrap_or_else(|| "default".to_string());
        pool_backend_map.entry(pool_name).or_default().push(backend);
    }

    // Parse algorithm
    let algo = match cfg.load_balancer.algorithm.to_ascii_lowercase().as_str() {
        "round_robin" => AlgorithmType::RoundRobin,
        "weighted_round_robin" => AlgorithmType::WeightedRoundRobin,
        "least_connections" => AlgorithmType::LeastConnections,
        "random" => AlgorithmType::Random,
        "ip_hash" => AlgorithmType::IpHash,
        "consistent_hash" => AlgorithmType::ConsistentHash,
        "power_of_two_choices" => AlgorithmType::PowerOfTwoChoices,
        "least_latency" => AlgorithmType::LeastLatency,
        "ewma_latency" => AlgorithmType::EwmaLatency,
        _ => AlgorithmType::Adaptive,
    };

    let mut pools = HashMap::new();
    for (name, pool_backends) in pool_backend_map {
        pools.insert(name.clone(), PoolGroup::new(name, pool_backends, algo));
    }

    // Build routes
    let mut routes = Vec::new();
    for r_cfg in &cfg.routes {
        let host = match &r_cfg.host {
            Some(h) if h.starts_with("*.") => HostMatch::Suffix(h[1..].to_string()),
            Some(h) => HostMatch::Exact(h.clone()),
            None => HostMatch::Any,
        };

        let path = if r_cfg.path.ends_with('*') {
            PathMatch::Prefix(r_cfg.path[..r_cfg.path.len() - 1].to_string())
        } else {
            PathMatch::Exact(r_cfg.path.clone())
        };

        routes.push(Route {
            name: r_cfg.name.clone(),
            host,
            path,
            methods: r_cfg.methods.clone(),
            headers: None,
            sni: r_cfg.sni.clone(),
            pool_name: r_cfg.pool.clone(),
            priority: r_cfg.priority,
        });
    }

    let default_pool_name = cfg
        .load_balancer
        .default_pool
        .as_deref()
        .or(Some("default"));
    let router = Arc::new(Router::new(routes, pools, default_pool_name));

    // 2. Build Dataplane auxiliary components
    let rate_limiter = Arc::new(RateLimiter::new(
        cfg.rate_limit.global_rps,
        cfg.rate_limit.client_rps,
    ));
    let conn_pool = ConnectionPool::new(ConnectionPoolConfig::default());
    let buffer_pool = BufferPool::default();
    let retry_policy = RetryPolicy::default();
    let http_cache = Arc::new(nexuslb_cache::HttpCache::default());

    // 3. Build TLS Acceptor if configured
    let tls_acceptor = if cfg.tls.enabled {
        let sni_resolver = Arc::new(DynamicSniResolver::new());
        if let (Some(cert_p), Some(key_p)) = (&cfg.tls.cert_path, &cfg.tls.key_path) {
            let cert_bytes = nexuslb_tls::load_pem_file(cert_p)?;
            let key_bytes = nexuslb_tls::load_pem_file(key_p)?;
            let certs = nexuslb_tls::parse_certs_from_pem(&cert_bytes)?;
            let key = nexuslb_tls::parse_key_from_pem(&key_bytes)?;
            sni_resolver
                .set_default_certificate(certs, key)
                .map_err(|e| anyhow::anyhow!(e))?;
        } else {
            let (cert_bytes, key_bytes) = nexuslb_tls::generate_self_signed(vec![
                "localhost".to_string(),
                "127.0.0.1".to_string(),
            ])?;
            let certs = nexuslb_tls::parse_certs_from_pem(&cert_bytes)?;
            let key = nexuslb_tls::parse_key_from_pem(&key_bytes)?;
            sni_resolver
                .set_default_certificate(certs, key)
                .map_err(|e| anyhow::anyhow!(e))?;
            info!("Generated self-signed TLS certificates for development/testing");
        }

        for (domain, sni_cfg) in &cfg.tls.sni {
            let cert_bytes = nexuslb_tls::load_pem_file(&sni_cfg.cert_path)?;
            let key_bytes = nexuslb_tls::load_pem_file(&sni_cfg.key_path)?;
            let certs = nexuslb_tls::parse_certs_from_pem(&cert_bytes)?;
            let key = nexuslb_tls::parse_key_from_pem(&key_bytes)?;
            sni_resolver
                .add_or_update_sni(domain, certs, key)
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        Some(TlsConfigBuilder::build_acceptor(sni_resolver).map_err(|e| anyhow::anyhow!(e))?)
    } else {
        None
    };

    // 4. Access Logger
    let (access_logger, _handle) = AccessLogger::new(
        cfg.access_log.enabled,
        &cfg.access_log.format,
        &cfg.access_log.target,
    );
    let access_logger = Arc::new(access_logger);

    // 5. Filter Chain
    let mut filter_chain = FilterChain::new();
    for route in &cfg.routes {
        if let Some(ref f) = route.filters {
            if f.jwt_secret.is_some() {
                let prefix = if route.path.ends_with('*') {
                    route.path[..route.path.len() - 1].to_string()
                } else {
                    route.path.clone()
                };
                filter_chain.add_filter(Arc::new(JwtAuthFilter::new(prefix)));
            }
            if !f.add_headers.is_empty() || !f.remove_headers.is_empty() {
                let mut rewrite = HeaderRewriteFilter::new();
                for (k, v) in &f.add_headers {
                    rewrite = rewrite.with_request_header(k, v);
                }
                for r in &f.remove_headers {
                    rewrite = rewrite.with_remove_header(r);
                }
                filter_chain.add_filter(Arc::new(rewrite));
            }
        }
    }

    let state = DataplaneState {
        router,
        rate_limiter,
        conn_pool,
        buffer_pool,
        retry_policy,
        tls_acceptor,
        http_cache,
        access_logger,
        filter_chain: Arc::new(filter_chain),
        redirect_http_to_https: cfg.tls.redirect_http_to_https,
    };

    Ok((state, backends))
}

async fn start_nexuslb(
    cfg: Arc<NexusConfig>,
    config_path: &str,
    engine_spec: &str,
    workers_spec: &str,
) -> anyhow::Result<()> {
    // 1. Build initial Dataplane state
    let (initial_state, backends) = build_dataplane_state(&cfg)?;
    let shared_state = Arc::new(SharedDataplaneState::new(initial_state));

    // 2. Worker metrics
    let num_workers = match workers_spec.trim().to_ascii_lowercase().as_str() {
        "auto" => std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4),
        other => other.parse::<usize>().unwrap_or(4),
    };
    let metrics = Arc::new(GlobalMetrics::new(num_workers));

    // 3. Hot reload handler
    let config_path_clone = config_path.to_string();
    let shared_state_clone = shared_state.clone();
    let reload_handler: nexuslb_api::ReloadHandler =
        Arc::new(move || match load_from_file(&config_path_clone) {
            Ok(new_cfg) => match build_dataplane_state(&new_cfg) {
                Ok((new_state, _)) => {
                    shared_state_clone.swap(new_state);
                    info!(
                        "Successfully reloaded configuration from {}",
                        config_path_clone
                    );
                    Ok(format!("Configuration reloaded from {}", config_path_clone))
                }
                Err(e) => {
                    error!("Failed to rebuild dataplane state on reload: {}", e);
                    Err(format!("Dataplane rebuild error: {}", e))
                }
            },
            Err(e) => {
                error!("Failed to load configuration file on reload: {}", e);
                Err(format!("Config file error: {}", e))
            }
        });

    // 4. Start active health checking tasks
    if cfg.health_check.enabled {
        let interval = nexuslb_config::parse_duration(&cfg.health_check.interval)
            .unwrap_or(Duration::from_secs(5));
        let timeout = nexuslb_config::parse_duration(&cfg.health_check.timeout)
            .unwrap_or(Duration::from_secs(2));

        for b in &backends {
            let check_type = if let Some(ref path) = cfg.health_check.http_path {
                HealthCheckType::Http {
                    path: path.clone(),
                    expected_status: cfg.health_check.expected_status,
                }
            } else {
                HealthCheckType::Tcp
            };

            let hc = Arc::new(ActiveHealthChecker::new(
                b.clone(),
                ActiveHealthCheckConfig {
                    check_type,
                    interval,
                    timeout,
                    healthy_threshold: cfg.health_check.healthy_threshold,
                    unhealthy_threshold: cfg.health_check.unhealthy_threshold,
                },
            ));
            tokio::spawn(async move {
                hc.run_loop().await;
            });
        }
    }

    // 5. Start Admin API server
    if cfg.admin.enabled {
        let admin_addr: SocketAddr = cfg.admin.address.parse()?;
        let admin = AdminServer::new(
            admin_addr,
            cfg.admin.token.clone(),
            metrics.clone(),
            shared_state.clone(),
            cfg.clone(),
        )
        .with_reloader(reload_handler.clone());

        tokio::spawn(async move {
            if let Err(e) = admin.run().await {
                error!(error = %e, "Admin API error");
            }
        });
    }

    // 5b. Start Prometheus metrics server
    if cfg.metrics.enabled {
        let metrics_addr: SocketAddr = cfg.metrics.address.parse()?;
        let metrics_ref = metrics.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            if let Ok(listener) = tokio::net::TcpListener::bind(metrics_addr).await {
                info!(address = %metrics_addr, "Prometheus metrics exporter listening");
                while let Ok((mut stream, _)) = listener.accept().await {
                    let text = metrics_ref.render_prometheus();
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        text.len(),
                        text
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                }
            }
        });
    }

    // 5c. Unix SIGHUP hot-reload listener
    #[cfg(unix)]
    {
        let sighup_reloader = reload_handler.clone();
        tokio::spawn(async move {
            use tokio::signal::unix::{signal, SignalKind};
            if let Ok(mut stream) = signal(SignalKind::hangup()) {
                while stream.recv().await.is_some() {
                    info!("Received SIGHUP signal, initiating zero-downtime hot reload...");
                    match sighup_reloader() {
                        Ok(msg) => info!("SIGHUP reload successful: {}", msg),
                        Err(e) => error!("SIGHUP reload failed: {}", e),
                    }
                }
            }
        });
    }

    // 5d. Dynamic Service Discovery
    if cfg.discovery.enabled {
        let interval = nexuslb_config::parse_duration(&cfg.discovery.interval)
            .unwrap_or(Duration::from_secs(10));
        let mut manager = nexuslb_discovery::DiscoveryManager::new();
        if cfg.discovery.provider.eq_ignore_ascii_case("file") {
            if let Some(ref source) = cfg.discovery.source {
                manager.add_provider(Box::new(nexuslb_discovery::FileCatalogDiscovery::new(
                    source,
                )));
            }
        }
        let manager = Arc::new(manager);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let instances = manager.sync_once().await;
                if !instances.is_empty() {
                    info!(
                        count = instances.len(),
                        "Dynamic service discovery synced instances"
                    );
                }
            }
        });
    }

    // 6. Listen addresses & socket config
    let mut listeners = Vec::new();
    for l in &cfg.server.listen {
        let addr: SocketAddr = l.parse()?;
        listeners.push(addr);
    }

    let socket_config = SocketConfig {
        reuse_port: cfg.server.reuse_port,
        tcp_nodelay: cfg.server.tcp_nodelay,
        recv_buffer_size: None,
        send_buffer_size: None,
        keepalive_idle: Some(Duration::from_secs(60)),
        keepalive_interval: Some(Duration::from_secs(10)),
        keepalive_retries: Some(3),
    };

    // 7. Dispatch selected I/O Engine
    match engine_spec.to_ascii_lowercase().as_str() {
        "auto" | "tokio" => {
            let mut engine = TokioEngine::new();
            engine.start(
                workers_spec,
                listeners,
                shared_state,
                metrics,
                socket_config,
            )?;

            info!("NexusLB is running. Press Ctrl+C to terminate.");
            signal::ctrl_c().await?;
            info!("Received termination signal. Shutting down...");
            engine.shutdown();
        }
        "io-uring" => {
            let mut engine = IoUringEngine::new();
            engine.start(
                workers_spec,
                listeners,
                shared_state,
                metrics,
                socket_config,
            )?;
        }
        "xdp" | "af-xdp" => {
            let mut engine = XdpEngine::new();
            engine.start(
                workers_spec,
                listeners,
                shared_state,
                metrics,
                socket_config,
            )?;
        }
        other => {
            anyhow::bail!(
                "Unknown I/O engine '{}'. Supported: auto, tokio, io-uring, xdp",
                other
            );
        }
    }

    Ok(())
}
