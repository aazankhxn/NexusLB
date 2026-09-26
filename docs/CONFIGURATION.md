# NexusLB Configuration Reference

NexusLB is configured using YAML. This document details all available configuration blocks, fields, default values, and operational guidelines.

---

## Complete Example `nexuslb.yaml`

```yaml
# ==========================================
# 1. CORE SERVER & RUNTIME ENGINE
# ==========================================
server:
  listen:
    - "0.0.0.0:80"
    - "0.0.0.0:443"
  workers: "auto"        # "auto" (detected CPU cores) or fixed integer like "8"
  engine: "tokio"        # "tokio", "io-uring" (Linux), or "xdp" (research)
  reuse_port: true       # SO_REUSEPORT socket kernel load balancing
  tcp_nodelay: true      # Disable Nagle's algorithm for microsecond latency
  max_connections: 500000

# ==========================================
# 2. LOAD BALANCER ALGORITHM & DEFAULTS
# ==========================================
load_balancer:
  algorithm: "adaptive"  # Options: adaptive, round_robin, weighted_round_robin,
                         # least_connections, random, ip_hash, consistent_hash,
                         # power_of_two_choices, least_latency, ewma_latency
  default_pool: "api-cluster"

# ==========================================
# 3. BACKEND DEFINITIONS
# ==========================================
backends:
  - name: "api-srv-1"
    address: "10.0.1.10:8080"
    weight: 100
    protocol: "http1"    # "http1", "http2", or "tcp"
    pool: "api-cluster"
    max_connections: 5000

  - name: "api-srv-2"
    address: "10.0.1.11:8080"
    weight: 100
    protocol: "http1"
    pool: "api-cluster"
    max_connections: 5000

  - name: "static-srv-1"
    address: "10.0.2.20:80"
    weight: 50
    protocol: "http1"
    pool: "static-cluster"

# ==========================================
# 4. ROUTING & FILTER PIPELINE
# ==========================================
routes:
  - name: "api-route"
    path: "/api/*"
    pool: "api-cluster"
    priority: 100
    methods:
      - "GET"
      - "POST"
      - "PUT"
      - "DELETE"
    filters:
      jwt_secret: "super-secret-key-or-token"
      add_headers:
        X-Proxy: "NexusLB"
        X-Edge-Cluster: "us-east-1"
      remove_headers:
        - "X-Internal-Token"

  - name: "static-route"
    path: "/static/*"
    pool: "static-cluster"
    priority: 50

# ==========================================
# 5. TLS TERMINATION & DYNAMIC SNI
# ==========================================
tls:
  enabled: true
  cert_path: "/etc/nexuslb/certs/default.crt"
  key_path: "/etc/nexuslb/certs/default.key"
  redirect_http_to_https: true # Issues 301 Moved Permanently for HTTP port 80
  sni:
    "api.example.com":
      cert_path: "/etc/nexuslb/certs/api.crt"
      key_path: "/etc/nexuslb/certs/api.key"
    "*.cdn.example.com":
      cert_path: "/etc/nexuslb/certs/wildcard.crt"
      key_path: "/etc/nexuslb/certs/wildcard.key"

# ==========================================
# 6. ACTIVE BACKGROUND HEALTH CHECKS
# ==========================================
health_check:
  enabled: true
  interval: "5s"
  timeout: "2s"
  healthy_threshold: 2     # Consecutive successes to mark UP
  unhealthy_threshold: 3   # Consecutive failures to mark DOWN
  http_path: "/healthz"    # Leave empty for pure TCP handshake check
  expected_status: 200

# ==========================================
# 7. CIRCUIT BREAKER
# ==========================================
circuit_breaker:
  enabled: true
  failure_threshold: 5     # Consecutive errors before tripping OPEN
  success_threshold: 3     # Consecutive successes in HALF-OPEN to close
  cool_down: "10s"         # Time before entering HALF-OPEN

# ==========================================
# 8. RATE LIMITING
# ==========================================
rate_limit:
  enabled: true
  global_rps: 150000       # Global rate limit across all clients
  client_rps: 2000         # Per-client IP sliding window rate limit

# ==========================================
# 9. STRUCTURED ACCESS LOGGING
# ==========================================
access_log:
  enabled: true
  format: "json"           # "json" or "combined"
  target: "stdout"         # "stdout", "stderr", or file path "/var/log/nexuslb/access.log"

# ==========================================
# 10. DYNAMIC SERVICE DISCOVERY
# ==========================================
discovery:
  enabled: false
  provider: "file"         # "file", "dns", or "k8s"
  interval: "10s"
  source: "/etc/nexuslb/discovery/backends.json"

# ==========================================
# 11. OBSERVABILITY & ADMIN REST API
# ==========================================
metrics:
  enabled: true
  address: "0.0.0.0:9090"  # Prometheus metrics endpoint at http://0.0.0.0:9090/

admin:
  enabled: true
  address: "127.0.0.1:9091"
  token: null              # Optional Bearer token for authentication
```

---

## Admin REST Endpoints Reference

When `admin.enabled: true` is configured, NexusLB exposes a fast, low-overhead HTTP management API:

| Endpoint | Method | Description |
| :--- | :--- | :--- |
| `/health` | `GET` | Health check returning `{"status":"healthy","service":"nexuslb"}`. |
| `/ready` | `GET` | Readiness check: `200 OK` if at least 1 backend is healthy, `503` otherwise. |
| `/metrics` | `GET` | Formatted Prometheus text metrics exposition. |
| `/backends` | `GET` | Complete JSON snapshot of all registered backends, states, and counters. |
| `/backends/{id}` | `GET` | Detailed telemetry and stats for a single backend by numeric ID. |
| `/backends/{id}/drain` | `POST` | Initiates graceful connection draining (30s window) before maintenance. |
| `/config` | `GET` | Returns active runtime configuration JSON. |
| `/reload` | `POST` | Triggers zero-downtime hot reload of the configuration file. |
