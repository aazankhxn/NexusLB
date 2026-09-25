use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, trace};

use nexuslb_config::NexusConfig;
use nexuslb_dataplane::SharedDataplaneState;
use nexuslb_health::DrainController;
use nexuslb_metrics::GlobalMetrics;

pub struct AdminServer {
    addr: SocketAddr,
    token: Option<String>,
    metrics: Arc<GlobalMetrics>,
    state: Arc<SharedDataplaneState>,
    config: Arc<NexusConfig>,
}

impl AdminServer {
    pub fn new(
        addr: SocketAddr,
        token: Option<String>,
        metrics: Arc<GlobalMetrics>,
        state: Arc<SharedDataplaneState>,
        config: Arc<NexusConfig>,
    ) -> Self {
        Self {
            addr,
            token,
            metrics,
            state,
            config,
        }
    }

    pub async fn run(self) -> std::io::Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        info!(address = %self.addr, "Admin API listening");

        let metrics = self.metrics.clone();
        let state = self.state.clone();
        let token = self.token.clone();
        let config = self.config.clone();

        loop {
            let (stream, client_addr) = listener.accept().await?;
            let metrics = metrics.clone();
            let state = state.clone();
            let token = token.clone();
            let config = config.clone();

            tokio::spawn(async move {
                if let Err(e) =
                    Self::handle_client(stream, client_addr, metrics, state, token, config).await
                {
                    trace!(error = %e, "Admin client connection terminated");
                }
            });
        }
    }

    async fn handle_client(
        mut stream: TcpStream,
        _client_addr: SocketAddr,
        metrics: Arc<GlobalMetrics>,
        state: Arc<SharedDataplaneState>,
        expected_token: Option<String>,
        config: Arc<NexusConfig>,
    ) -> std::io::Result<()> {
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }

        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut req = httparse::Request::new(&mut headers);

        let (method, path) = match req.parse(&buf[..n]) {
            Ok(httparse::Status::Complete(_)) => {
                let method = req.method.unwrap_or("GET");
                let path = req.path.unwrap_or("/");

                // Check authorization if token is configured
                if let Some(ref required_token) = expected_token {
                    let authorized = req.headers.iter().any(|h| {
                        if h.name.eq_ignore_ascii_case("authorization") {
                            let val = String::from_utf8_lossy(h.value);
                            val.trim() == format!("Bearer {}", required_token)
                        } else {
                            false
                        }
                    });

                    if !authorized {
                        let resp =
                            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 13\r\n\r\nUnauthorized\n";
                        stream.write_all(resp.as_bytes()).await?;
                        return Ok(());
                    }
                }

                (method.to_string(), path.to_string())
            }
            _ => {
                let resp = "HTTP/1.1 400 Bad Request\r\nContent-Length: 11\r\n\r\nBad Request";
                stream.write_all(resp.as_bytes()).await?;
                return Ok(());
            }
        };

        // Dispatch endpoints
        let (status, content_type, body) = match (method.as_str(), path.as_str()) {
            ("GET", "/health") => (
                "200 OK",
                "application/json",
                r#"{"status":"healthy","service":"nexuslb"}"#.to_string(),
            ),
            ("GET", "/ready") => {
                let current_state = state.load();
                let has_healthy = current_state
                    .router
                    .pools()
                    .values()
                    .any(|p| p.healthy_count() > 0);
                if has_healthy {
                    (
                        "200 OK",
                        "application/json",
                        r#"{"ready":true}"#.to_string(),
                    )
                } else {
                    (
                        "503 Service Unavailable",
                        "application/json",
                        r#"{"ready":false,"reason":"no healthy backends"}"#.to_string(),
                    )
                }
            }
            ("GET", "/metrics") => {
                let prom_text = metrics.render_prometheus();
                ("200 OK", "text/plain; version=0.0.4", prom_text)
            }
            ("GET", "/backends") => {
                let current_state = state.load();
                let mut snapshots = Vec::new();
                for pool in current_state.router.pools().values() {
                    for b in &pool.backends {
                        snapshots.push(b.snapshot());
                    }
                }
                let json =
                    serde_json::to_string_pretty(&snapshots).unwrap_or_else(|_| "[]".to_string());
                ("200 OK", "application/json", json)
            }
            ("GET", p) if p.starts_with("/backends/") => {
                let id_str = &p["/backends/".len()..];
                let current_state = state.load();
                let mut found = None;

                if let Ok(id_num) = id_str.parse::<u64>() {
                    for pool in current_state.router.pools().values() {
                        for b in &pool.backends {
                            if b.id().0 == id_num {
                                found = Some(b.snapshot());
                                break;
                            }
                        }
                    }
                }

                if let Some(snap) = found {
                    let json = serde_json::to_string_pretty(&snap).unwrap_or_default();
                    ("200 OK", "application/json", json)
                } else {
                    (
                        "404 Not Found",
                        "application/json",
                        r#"{"error":"Backend not found"}"#.to_string(),
                    )
                }
            }
            ("POST", p) if p.ends_with("/drain") => {
                let id_part = &p["/backends/".len()..p.len() - "/drain".len()];
                let current_state = state.load();
                let mut target = None;

                if let Ok(id_num) = id_part.parse::<u64>() {
                    for pool in current_state.router.pools().values() {
                        for b in &pool.backends {
                            if b.id().0 == id_num {
                                target = Some(b.clone());
                                break;
                            }
                        }
                    }
                }

                if let Some(b) = target {
                    tokio::spawn(async move {
                        DrainController::drain_backend(b, std::time::Duration::from_secs(30)).await;
                    });
                    (
                        "202 Accepted",
                        "application/json",
                        r#"{"status":"draining started"}"#.to_string(),
                    )
                } else {
                    (
                        "404 Not Found",
                        "application/json",
                        r#"{"error":"Backend not found"}"#.to_string(),
                    )
                }
            }
            ("GET", "/config") => {
                let json = serde_json::to_string_pretty(&*config).unwrap_or_default();
                ("200 OK", "application/json", json)
            }
            ("POST", "/reload") => (
                "200 OK",
                "application/json",
                r#"{"status":"reload requested"}"#.to_string(),
            ),
            _ => (
                "404 Not Found",
                "application/json",
                r#"{"error":"Endpoint not found"}"#.to_string(),
            ),
        };

        let response = format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status,
            content_type,
            body.len(),
            body
        );

        stream.write_all(response.as_bytes()).await?;
        Ok(())
    }
}
