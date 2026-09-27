use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusConfig {
    pub server: ServerConfig,
    #[serde(default)]
    pub load_balancer: LoadBalancerConfig,
    pub backends: Vec<BackendConfig>,
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
    #[serde(default)]
    pub health_check: HealthCheckConfig,
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub admin: AdminConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub access_log: AccessLogConfig,
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub listen: Vec<String>,
    #[serde(default = "default_workers")]
    pub workers: String, // "auto" or number
    #[serde(default = "default_engine")]
    pub engine: String, // "auto", "tokio", "io-uring", "xdp", "af-xdp"
    #[serde(default = "default_reuse_port")]
    pub reuse_port: bool,
    #[serde(default = "default_tcp_nodelay")]
    pub tcp_nodelay: bool,
    #[serde(default)]
    pub max_connections: Option<u64>,
}

fn default_workers() -> String {
    "auto".to_string()
}

fn default_engine() -> String {
    "auto".to_string()
}

fn default_reuse_port() -> bool {
    true
}

fn default_tcp_nodelay() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    #[serde(default = "default_algorithm")]
    pub algorithm: String, // "adaptive", "round_robin", etc.
    #[serde(default)]
    pub default_pool: Option<String>,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            algorithm: default_algorithm(),
            default_pool: None,
        }
    }
}

fn default_algorithm() -> String {
    "adaptive".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub name: String,
    pub address: String,
    #[serde(default = "default_weight")]
    pub weight: u32,
    #[serde(default = "default_protocol")]
    pub protocol: String, // "tcp", "http1", "http2"
    #[serde(default)]
    pub max_connections: Option<u64>,
    #[serde(default)]
    pub pool: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_weight() -> u32 {
    100
}

fn default_protocol() -> String {
    "http1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub name: String,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default)]
    pub methods: Option<Vec<String>>,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub sni: Option<String>,
    pub pool: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub filters: Option<FilterConfig>,
}

fn default_path() -> String {
    "/".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_interval")]
    pub interval: String,
    #[serde(default = "default_timeout")]
    pub timeout: String,
    #[serde(default = "default_healthy_thresh")]
    pub healthy_threshold: u32,
    #[serde(default = "default_unhealthy_thresh")]
    pub unhealthy_threshold: u32,
    #[serde(default)]
    pub http_path: Option<String>,
    #[serde(default = "default_expected_status")]
    pub expected_status: u16,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: default_interval(),
            timeout: default_timeout(),
            healthy_threshold: default_healthy_thresh(),
            unhealthy_threshold: default_unhealthy_thresh(),
            http_path: None,
            expected_status: default_expected_status(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_interval() -> String {
    "5s".to_string()
}

fn default_timeout() -> String {
    "2s".to_string()
}

fn default_healthy_thresh() -> u32 {
    2
}

fn default_unhealthy_thresh() -> u32 {
    3
}

fn default_expected_status() -> u16 {
    200
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_failure_thresh")]
    pub failure_threshold: u32,
    #[serde(default = "default_success_thresh")]
    pub success_threshold: u32,
    #[serde(default = "default_cooldown")]
    pub cool_down: String,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            failure_threshold: default_failure_thresh(),
            success_threshold: default_success_thresh(),
            cool_down: default_cooldown(),
        }
    }
}

fn default_failure_thresh() -> u32 {
    5
}

fn default_success_thresh() -> u32 {
    3
}

fn default_cooldown() -> String {
    "10s".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TlsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub cert_path: Option<String>,
    #[serde(default)]
    pub key_path: Option<String>,
    #[serde(default)]
    pub redirect_http_to_https: bool,
    #[serde(default)]
    pub sni: HashMap<String, SniConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniConfig {
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_metrics_addr")]
    pub address: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            address: default_metrics_addr(),
        }
    }
}

fn default_metrics_addr() -> String {
    "127.0.0.1:9090".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_admin_addr")]
    pub address: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub mutation_token: Option<String>,
    #[serde(default)]
    pub authentication: AdminAuthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAuthConfig {
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default = "default_true")]
    pub allow_unauthenticated_health: bool,
}

impl Default for AdminAuthConfig {
    fn default() -> Self {
        Self {
            required: true,
            allow_unauthenticated_health: true,
        }
    }
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            address: default_admin_addr(),
            token: None,
            mutation_token: None,
            authentication: AdminAuthConfig::default(),
        }
    }
}

fn default_admin_addr() -> String {
    "127.0.0.1:9091".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    #[serde(default = "default_max_header_size")]
    pub max_header_size: usize,
    #[serde(default = "default_max_body_size")]
    pub max_request_body_size: usize,
    #[serde(default = "default_max_conns")]
    pub max_connections: usize,
    #[serde(default = "default_h2_max_concurrent_streams")]
    pub h2_max_concurrent_streams: u32,
    #[serde(default = "default_h2_stream_timeout_secs")]
    pub h2_stream_timeout_secs: u64,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_header_size: default_max_header_size(),
            max_request_body_size: default_max_body_size(),
            max_connections: default_max_conns(),
            h2_max_concurrent_streams: default_h2_max_concurrent_streams(),
            h2_stream_timeout_secs: default_h2_stream_timeout_secs(),
        }
    }
}

fn default_max_header_size() -> usize {
    64 * 1024 // 64 KB
}

fn default_max_body_size() -> usize {
    16 * 1024 * 1024 // 16 MB
}

fn default_max_conns() -> usize {
    100_000
}

fn default_h2_max_concurrent_streams() -> u32 {
    128
}

fn default_h2_stream_timeout_secs() -> u64 {
    30
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub global_rps: Option<u32>,
    #[serde(default)]
    pub client_rps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLogConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_access_log_format")]
    pub format: String, // "json" | "combined"
    #[serde(default = "default_access_log_target")]
    pub target: String, // "stdout" | "stderr" | file path
}

impl Default for AccessLogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            format: default_access_log_format(),
            target: default_access_log_target(),
        }
    }
}

fn default_access_log_format() -> String {
    "json".to_string()
}

fn default_access_log_target() -> String {
    "stdout".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_discovery_provider")]
    pub provider: String, // "file" | "dns" | "k8s"
    #[serde(default = "default_discovery_interval")]
    pub interval: String,
    #[serde(default)]
    pub source: Option<String>,
}

fn default_discovery_provider() -> String {
    "file".to_string()
}

fn default_discovery_interval() -> String {
    "10s".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilterConfig {
    #[serde(default)]
    pub jwt_secret: Option<String>,
    #[serde(default)]
    pub add_headers: HashMap<String, String>,
    #[serde(default)]
    pub remove_headers: Vec<String>,
}

impl NexusConfig {
    /// Return a sanitized, redacted copy of configuration safe for public inspection via the API.
    pub fn to_redacted(&self) -> Self {
        let mut redacted = self.clone();

        // 1. Redact admin auth tokens
        if redacted.admin.token.is_some() {
            redacted.admin.token = Some("[REDACTED]".to_string());
        }
        if redacted.admin.mutation_token.is_some() {
            redacted.admin.mutation_token = Some("[REDACTED]".to_string());
        }

        // 2. Redact TLS private key paths
        if redacted.tls.key_path.is_some() {
            redacted.tls.key_path = Some("[REDACTED]".to_string());
        }
        for sni in redacted.tls.sni.values_mut() {
            sni.key_path = "[REDACTED]".to_string();
        }

        // 3. Redact route filter secrets (JWT tokens, HMAC keys)
        for route in &mut redacted.routes {
            if let Some(ref mut filters) = route.filters {
                if filters.jwt_secret.is_some() {
                    filters.jwt_secret = Some("[REDACTED]".to_string());
                }
            }
        }

        redacted
    }
}
