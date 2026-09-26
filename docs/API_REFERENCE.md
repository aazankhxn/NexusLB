# NexusLB Admin REST API & Observability Reference

NexusLB includes a built-in, non-blocking administrative HTTP REST API running independently of the traffic proxy dataplane (default port `9091`). It allows operators, CI/CD systems, and orchestration platforms (Kubernetes, Nomad) to inspect health, query live backend metrics, drain servers gracefully, and trigger atomic zero-downtime hot reloads.

---

## 🔐 Administrative Endpoints Overview

| Method | Endpoint | Description | Auth Required |
| :--- | :--- | :--- | :--- |
| `GET` | `/health` | Liveness probe returning `200 OK` if the process is responsive | No |
| `GET` | `/ready` | Readiness probe returning `200 OK` if $\ge 1$ upstream backend is healthy | No |
| `GET` | `/metrics` | Prometheus metrics scrape exposition format | Optional |
| `GET` | `/backends` | Full JSON dump of all pools, backends, active connections, and latency | Optional |
| `GET` | `/backends/:id` | Detailed metrics and health state for a single backend | Optional |
| `POST` | `/backends/:id/drain` | Initiate graceful connection drain for maintenance | Yes (Bearer) |
| `POST` | `/backends/:id/undrain` | Re-enable traffic forwarding to a drained backend | Yes (Bearer) |
| `GET` | `/config` | Dumps the active in-memory configuration schema | Optional |
| `POST` | `/reload` | Atomic zero-downtime reload of `nexuslb.yaml` via ArcSwap (< 1 µs) | Yes (Bearer) |

---

## 📖 Endpoint Specifications & Examples

### 1. `GET /health` (Liveness)
Returns immediate status indicating the Tokio event loop and memory subsystems are responsive.
```bash
curl -i http://127.0.0.1:9091/health
```
**Response (`200 OK`):**
```json
{
  "status": "healthy",
  "version": "0.0.1",
  "uptime_seconds": 128492,
  "engine": "tokio",
  "workers": 8
}
```

---

### 2. `GET /ready` (Readiness)
Evaluates upstream cluster availability. Returns `200 OK` if the proxy has at least one active, healthy backend ready to accept traffic; returns `503 Service Unavailable` if all backends are down or quarantined by circuit breakers.
```bash
curl -i http://127.0.0.1:9091/ready
```
**Response (`200 OK`):**
```json
{
  "ready": true,
  "healthy_backends": 6,
  "total_backends": 6,
  "active_pools": ["api-cluster", "static-cluster"]
}
```

---

### 3. `GET /backends` (Cluster State & Live Telemetry)
Returns high-resolution telemetry across all configured upstream server instances.
```bash
curl -s http://127.0.0.1:9091/backends | jq .
```
**Response (`200 OK`):**
```json
{
  "pools": {
    "api-cluster": {
      "algorithm": "Adaptive",
      "backends": [
        {
          "id": "node-01",
          "address": "10.0.1.10:8080",
          "weight": 100,
          "state": "Up",
          "active_connections": 42,
          "total_requests": 1492040,
          "failed_requests": 3,
          "latency": {
            "p50_us": 480,
            "p90_us": 1240,
            "p99_us": 2350,
            "ewma_us": 512
          },
          "circuit_breaker": {
            "state": "Closed",
            "consecutive_errors": 0,
            "trips_total": 0
          }
        },
        {
          "id": "node-02",
          "address": "10.0.1.11:8080",
          "weight": 100,
          "state": "Up",
          "active_connections": 38,
          "total_requests": 1488102,
          "failed_requests": 1,
          "latency": {
            "p50_us": 495,
            "p90_us": 1190,
            "p99_us": 2210,
            "ewma_us": 508
          },
          "circuit_breaker": {
            "state": "Closed",
            "consecutive_errors": 0,
            "trips_total": 0
          }
        }
      ]
    }
  }
}
```

---

### 4. `POST /backends/:id/drain` (Graceful Backend Draining)
Instructs NexusLB to gracefully isolate a backend server before taking it offline for operating system updates, kernel patches, or application deployment. NexusLB immediately stops routing new client connections to this node, while allowing in-flight requests to complete without dropping a single packet.
```bash
curl -X POST http://127.0.0.1:9091/backends/node-01/drain \
  -H "Authorization: Bearer secret-admin-token" \
  -H "Content-Type: application/json" \
  -d '{"timeout_seconds": 60}'
```
**Response (`200 OK`):**
```json
{
  "backend_id": "node-01",
  "status": "Draining",
  "remaining_active_connections": 14,
  "drain_deadline_epoch": 1790416800
}
```

---

### 5. `POST /reload` (Atomic Zero-Downtime Configuration Reload)
Re-parses `nexuslb.yaml`, validates upstream addresses, reloads SSL/TLS certificates and private keys from disk, and swaps the global atomic routing pointer in under **1 microsecond** using `arc-swap`. Client connections currently in-flight continue processing on their existing state without interruption.
```bash
curl -X POST http://127.0.0.1:9091/reload \
  -H "Authorization: Bearer secret-admin-token"
```
**Response (`200 OK`):**
```json
{
  "success": true,
  "message": "Configuration reloaded successfully from /etc/nexuslb/nexuslb.yaml",
  "swap_duration_us": 0.8,
  "pools_loaded": 3,
  "routes_loaded": 5
}
```

---

## 📊 Prometheus Metrics Reference (`GET /metrics`)

All metrics are emitted using cacheline-padded 64-bit atomic registers (`AtomicU64`) without holding mutexes during request forwarding:

| Metric Name | Type | Description |
| :--- | :--- | :--- |
| `nexuslb_requests_total` | Counter | Total HTTP requests ingested by listener port |
| `nexuslb_active_connections` | Gauge | Current number of active client TCP sockets |
| `nexuslb_backend_requests_total` | Counter | Requests forwarded to upstreams, partitioned by `{pool, backend}` |
| `nexuslb_backend_errors_total` | Counter | Total upstream connection errors, resets, and timeouts |
| `nexuslb_request_duration_seconds` | Histogram | High-resolution latency histogram (0.1ms to 10s buckets) |
| `nexuslb_circuit_breaker_trips_total` | Counter | Number of times a circuit breaker entered `Open` state |
| `nexuslb_bytes_received_total` | Counter | Total ingress network bandwidth in bytes |
| `nexuslb_bytes_sent_total` | Counter | Total egress network bandwidth in bytes |
| `nexuslb_buffer_pool_allocated` | Gauge | Active buffers in use from the lock-free `BufferPool` |
| `nexuslb_buffer_pool_capacity` | Gauge | Maximum pre-allocated buffer capacity in memory |

---

## 🖥️ Terminal Operator TUI (`nexuslb top`)

In addition to REST endpoints, NexusLB ships with an interactive real-time terminal dashboard running at 10 Hz refresh rates:

```bash
# Connect to local or remote instance
nexuslb top --admin-addr 127.0.0.1:9091
```

### Keybindings:
- **`q` or `Esc`:** Exit dashboard.
- **`Tab`:** Cycle between active upstream pools (`api-cluster`, `media-cluster`).
- **`d`:** Trigger instant drain for the currently highlighted node.
- **`r`:** Request manual configuration reload.
- **`Space`:** Toggle sorting order (Throughput, Active Connections, P99 Latency, Composite Score).
