use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;
use tracing::{error, info, trace, warn};

use nexuslb_core::backend::Backend;
use nexuslb_core::types::BackendState;

#[derive(Debug, Clone)]
pub enum HealthCheckType {
    Tcp,
    Http { path: String, expected_status: u16 },
}

#[derive(Debug, Clone)]
pub struct ActiveHealthCheckConfig {
    pub check_type: HealthCheckType,
    pub interval: Duration,
    pub timeout: Duration,
    pub healthy_threshold: u32,
    pub unhealthy_threshold: u32,
}

impl Default for ActiveHealthCheckConfig {
    fn default() -> Self {
        Self {
            check_type: HealthCheckType::Tcp,
            interval: Duration::from_secs(5),
            timeout: Duration::from_secs(2),
            healthy_threshold: 2,
            unhealthy_threshold: 3,
        }
    }
}

pub struct ActiveHealthChecker {
    backend: Arc<Backend>,
    config: ActiveHealthCheckConfig,
    consecutive_successes: AtomicU32,
    consecutive_failures: AtomicU32,
}

impl ActiveHealthChecker {
    pub fn new(backend: Arc<Backend>, config: ActiveHealthCheckConfig) -> Self {
        Self {
            backend,
            config,
            consecutive_successes: AtomicU32::new(0),
            consecutive_failures: AtomicU32::new(0),
        }
    }

    pub async fn run_loop(self: Arc<Self>) {
        loop {
            sleep(self.config.interval).await;

            // If backend is draining, do not touch health check
            if self.backend.state().is_draining() {
                continue;
            }

            let result = self.execute_probe().await;
            match result {
                Ok(_) => {
                    self.consecutive_failures.store(0, Ordering::Relaxed);
                    let succ = self.consecutive_successes.fetch_add(1, Ordering::Relaxed) + 1;
                    trace!(backend = %self.backend.name(), successes = succ, "Health probe succeeded");

                    if succ >= self.config.healthy_threshold
                        && self.backend.state() != BackendState::Up
                    {
                        self.backend.set_state(BackendState::Up);
                        info!(
                            backend_id = %self.backend.id(),
                            name = %self.backend.name(),
                            "Backend marked UP by active health checker"
                        );
                    }
                }
                Err(err) => {
                    self.consecutive_successes.store(0, Ordering::Relaxed);
                    let fails = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                    warn!(
                        backend = %self.backend.name(),
                        failures = fails,
                        error = %err,
                        "Health probe failed"
                    );

                    if fails >= self.config.unhealthy_threshold
                        && self.backend.state() == BackendState::Up
                    {
                        self.backend.set_state(BackendState::Down);
                        error!(
                            backend_id = %self.backend.id(),
                            name = %self.backend.name(),
                            "Backend marked DOWN by active health checker"
                        );
                    }
                }
            }
        }
    }

    async fn execute_probe(&self) -> Result<(), String> {
        let addr = self.backend.socket_addr();

        match &self.config.check_type {
            HealthCheckType::Tcp => {
                match tokio::time::timeout(self.config.timeout, TcpStream::connect(addr)).await {
                    Ok(Ok(_stream)) => Ok(()),
                    Ok(Err(e)) => Err(format!("TCP connect error: {}", e)),
                    Err(_) => Err("TCP connect timeout".to_string()),
                }
            }
            HealthCheckType::Http {
                path,
                expected_status,
            } => {
                match tokio::time::timeout(self.config.timeout, TcpStream::connect(addr)).await {
                    Ok(Ok(mut stream)) => {
                        let request = format!(
                            "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: NexusLB-HealthCheck/1.0\r\n\r\n",
                            path, addr
                        );
                        if let Err(e) = stream.write_all(request.as_bytes()).await {
                            return Err(format!("Write error: {}", e));
                        }

                        let mut buf = [0u8; 512];
                        let n = match stream.read(&mut buf).await {
                            Ok(n) if n > 0 => n,
                            Ok(_) => return Err("Unexpected EOF from backend".to_string()),
                            Err(e) => return Err(format!("Read error: {}", e)),
                        };

                        let response_str = String::from_utf8_lossy(&buf[..n]);
                        // Parse status line: HTTP/1.1 200 OK
                        let status_code = response_str
                            .lines()
                            .next()
                            .and_then(|line| {
                                let mut parts = line.split_whitespace();
                                parts.next()?; // HTTP/1.1
                                parts.next()?.parse::<u16>().ok()
                            })
                            .unwrap_or(0);

                        if status_code == *expected_status || (200..400).contains(&status_code) {
                            Ok(())
                        } else {
                            Err(format!("Unexpected HTTP status code: {}", status_code))
                        }
                    }
                    Ok(Err(e)) => Err(format!("TCP connect error: {}", e)),
                    Err(_) => Err("Probe timeout".to_string()),
                }
            }
        }
    }
}
