use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, trace};

use nexuslb_config::NexusConfig;
use nexuslb_dataplane::SharedDataplaneState;
use nexuslb_health::DrainController;
use nexuslb_metrics::GlobalMetrics;

pub type ReloadHandler = Arc<dyn Fn() -> Result<String, String> + Send + Sync>;

/// Constant-time byte comparison to eliminate side-channel timing attacks.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub struct AdminServer {
    addr: SocketAddr,
    token: Option<String>,
    mutation_token: Option<String>,
    metrics: Arc<GlobalMetrics>,
    state: Arc<SharedDataplaneState>,
    config: Arc<NexusConfig>,
    reload_handler: Option<ReloadHandler>,
}

impl AdminServer {
    pub fn new(
        addr: SocketAddr,
        token: Option<String>,
        metrics: Arc<GlobalMetrics>,
        state: Arc<SharedDataplaneState>,
        config: Arc<NexusConfig>,
    ) -> Self {
        let mutation_token = config.admin.mutation_token.clone();
        Self {
            addr,
            token,
            mutation_token,
            metrics,
            state,
            config,
            reload_handler: None,
        }
    }

    pub fn with_reloader(mut self, reloader: ReloadHandler) -> Self {
        self.reload_handler = Some(reloader);
        self
    }

    pub async fn run(self) -> std::io::Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        info!(address = %self.addr, "Admin API listening with secure authentication");

        let metrics = self.metrics.clone();
        let state = self.state.clone();
        let token = self.token.clone();
        let mutation_token = self.mutation_token.clone();
        let config = self.config.clone();
        let reload_handler = self.reload_handler.clone();

        loop {
            let (stream, client_addr) = listener.accept().await?;
            let metrics = metrics.clone();
            let state = state.clone();
            let token = token.clone();
            let mutation_token = mutation_token.clone();
            let config = config.clone();
            let reload_handler = reload_handler.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_client(
                    stream,
                    client_addr,
                    metrics,
                    state,
                    token,
                    mutation_token,
                    config,
                    reload_handler,
                )
                .await
                {
                    trace!(error = %e, "Admin client connection terminated");
                }
            });
        }
    }

    fn extract_bearer_token<'a>(headers: &'a [httparse::Header<'a>]) -> Option<&'a str> {
        for h in headers {
            if h.name.eq_ignore_ascii_case("authorization") {
                if let Ok(val) = std::str::from_utf8(h.value) {
                    let trimmed = val.trim();
                    if trimmed.starts_with("Bearer ") || trimmed.starts_with("bearer ") {
                        return Some(trimmed[7..].trim());
                    }
                }
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_client(
        mut stream: TcpStream,
        _client_addr: SocketAddr,
        metrics: Arc<GlobalMetrics>,
        state: Arc<SharedDataplaneState>,
        expected_token: Option<String>,
        mutation_token: Option<String>,
        config: Arc<NexusConfig>,
        reload_handler: Option<ReloadHandler>,
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

                let is_mutation = method.eq_ignore_ascii_case("POST")
                    || method.eq_ignore_ascii_case("PUT")
                    || method.eq_ignore_ascii_case("DELETE");

                let is_health_probe = (path == "/health" || path == "/ready")
                    && config.admin.authentication.allow_unauthenticated_health;

                // Check authorization when required
                if !is_health_probe && config.admin.authentication.required {
                    let provided_token = Self::extract_bearer_token(req.headers);

                    match provided_token {
                        None => {
                            let resp = "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: 42\r\n\r\n{\"error\":\"Authorization header required\"}\n";
                            stream.write_all(resp.as_bytes()).await?;
                            return Ok(());
                        }
                        Some(token_str) => {
                            let target_expected = if is_mutation {
                                mutation_token.as_ref().or(expected_token.as_ref())
                            } else {
                                expected_token.as_ref().or(mutation_token.as_ref())
                            };

                            let authorized = match target_expected {
                                Some(expected) => {
                                    constant_time_eq(token_str.as_bytes(), expected.as_bytes())
                                }
                                None => false,
                            };

                            if !authorized {
                                let resp = if is_mutation {
                                    "HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\nContent-Length: 42\r\n\r\n{\"error\":\"Insufficient mutation privileges\"}\n"
                                } else {
                                    "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: 31\r\n\r\n{\"error\":\"Invalid admin token\"}\n"
                                };
                                stream.write_all(resp.as_bytes()).await?;
                                return Ok(());
                            }
                        }
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
                // Redact sensitive secrets (JWT secrets, private keys, admin tokens)
                let redacted = config.to_redacted();
                let json = serde_json::to_string_pretty(&redacted).unwrap_or_default();
                ("200 OK", "application/json", json)
            }
            ("POST", "/reload") => {
                if let Some(ref reloader) = reload_handler {
                    match reloader() {
                        Ok(msg) => (
                            "200 OK",
                            "application/json",
                            format!(r#"{{"status":"success","message":"{}"}}"#, msg),
                        ),
                        Err(e) => (
                            "500 Internal Server Error",
                            "application/json",
                            format!(r#"{{"status":"error","error":"{}"}}"#, e),
                        ),
                    }
                } else {
                    (
                        "200 OK",
                        "application/json",
                        r#"{"status":"reload requested"}"#.to_string(),
                    )
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"my-secret-token", b"my-secret-token"));
        assert!(!constant_time_eq(b"my-secret-token", b"wrong-token-abc"));
        assert!(!constant_time_eq(b"short", b"longer-token"));
    }
}
