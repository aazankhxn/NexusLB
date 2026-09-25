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
    "0.0.0.0:9090".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_admin_addr")]
    pub address: String,
    #[serde(default)]
    pub token: Option<String>,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            address: default_admin_addr(),
            token: None,
        }
    }
}

fn default_admin_addr() -> String {
    "127.0.0.1:9091".to_string()
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
