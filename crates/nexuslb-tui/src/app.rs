use serde::Deserialize;
use std::collections::VecDeque;
use std::time::Instant;

#[derive(Debug, Clone, Deserialize)]
pub struct BackendStatsDto {
    pub active_connections: u64,
    pub total_connections: u64,
    pub total_requests: u64,
    pub total_responses: u64,
    pub total_errors: u64,
    pub consecutive_errors: u32,
    pub consecutive_successes: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub avg_latency_micros: u64,
    pub ewma_latency_micros: u64,
    pub min_latency_micros: u64,
    pub max_latency_micros: u64,
    pub score: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackendDto {
    pub id: u64,
    pub name: String,
    pub address: String,
    pub weight: u32,
    pub state: String,
    pub circuit: String,
    pub protocol: String,
    pub max_connections: Option<u64>,
    pub stats: BackendStatsDto,
}

pub struct App {
    pub admin_url: String,
    pub client: reqwest::Client,
    pub backends: Vec<BackendDto>,
    pub selected_index: usize,
    pub throughput_history: VecDeque<u64>,
    pub latency_history: VecDeque<u64>,
    pub last_total_requests: u64,
    pub last_poll_instant: Instant,
    pub status_message: String,
    pub is_running: bool,
}

impl App {
    pub fn new(admin_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(800))
            .build()
            .unwrap_or_default();

        Self {
            admin_url,
            client,
            backends: Vec::new(),
            selected_index: 0,
            throughput_history: VecDeque::from(vec![0; 40]),
            latency_history: VecDeque::from(vec![0; 40]),
            last_total_requests: 0,
            last_poll_instant: Instant::now(),
            status_message: "Connected to NexusLB Admin API".to_string(),
            is_running: true,
        }
    }

    pub async fn poll_metrics(&mut self) {
        let backends_url = format!("{}/backends", self.admin_url.trim_end_matches('/'));
        match self.client.get(&backends_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(backends) = resp.json::<Vec<BackendDto>>().await {
                    let mut current_total_reqs = 0;
                    let mut total_ewma_latency = 0;
                    let mut active_backend_count = 0;

                    for b in &backends {
                        current_total_reqs += b.stats.total_requests;
                        if b.stats.ewma_latency_micros > 0 {
                            total_ewma_latency += b.stats.ewma_latency_micros;
                            active_backend_count += 1;
                        }
                    }

                    let elapsed = self.last_poll_instant.elapsed().as_secs_f64();
                    if self.last_total_requests > 0 && elapsed > 0.0 {
                        let delta_reqs =
                            current_total_reqs.saturating_sub(self.last_total_requests);
                        let qps = (delta_reqs as f64 / elapsed) as u64;
                        self.throughput_history.push_back(qps);
                        if self.throughput_history.len() > 50 {
                            self.throughput_history.pop_front();
                        }
                    }
                    self.last_total_requests = current_total_reqs;
                    self.last_poll_instant = Instant::now();

                    let avg_ewma = total_ewma_latency
                        .checked_div(active_backend_count)
                        .unwrap_or(0);
                    self.latency_history.push_back(avg_ewma);
                    if self.latency_history.len() > 50 {
                        self.latency_history.pop_front();
                    }

                    self.backends = backends;
                    if self.selected_index >= self.backends.len() && !self.backends.is_empty() {
                        self.selected_index = self.backends.len() - 1;
                    }
                }
            }
            Ok(resp) => {
                self.status_message = format!("Admin API returned HTTP {}", resp.status());
            }
            Err(e) => {
                self.status_message = format!("Polling error: {}", e);
            }
        }
    }

    pub async fn trigger_reload(&mut self) {
        let reload_url = format!("{}/reload", self.admin_url.trim_end_matches('/'));
        match self.client.post(&reload_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                self.status_message = "Configuration reloaded successfully".to_string();
            }
            Ok(resp) => {
                self.status_message = format!("Reload returned status: {}", resp.status());
            }
            Err(e) => {
                self.status_message = format!("Reload failed: {}", e);
            }
        }
    }

    pub async fn toggle_drain(&mut self) {
        if let Some(b) = self.backends.get(self.selected_index) {
            let drain_url = format!(
                "{}/backends/{}/drain",
                self.admin_url.trim_end_matches('/'),
                b.id
            );
            match self.client.post(&drain_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    self.status_message = format!("Toggled drain mode on backend '{}'", b.name);
                    self.poll_metrics().await;
                }
                Ok(resp) => {
                    self.status_message = format!("Drain action failed: status {}", resp.status());
                }
                Err(e) => {
                    self.status_message = format!("Drain request failed: {}", e);
                }
            }
        }
    }

    pub fn next_backend(&mut self) {
        if !self.backends.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.backends.len();
        }
    }

    pub fn previous_backend(&mut self) {
        if !self.backends.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.backends.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }
}
