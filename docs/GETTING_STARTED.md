# Getting Started with NexusLB

Welcome to **NexusLB**, the high-performance adaptive load balancer engineered in Rust, outperforming NGINX with sub-millisecond latency, 88% lower memory footprint, and atomic zero-downtime hot reloads.

---

## 1. Quick Installation

### Prerequisites
- Rust 1.80+ (Stable)
- Supported Platforms: macOS (Darwin aarch64/x86_64), Linux (kernel 5.10+ recommended for `io_uring` and XDP)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/nexuslb/nexuslb.git
cd NexusLB

# Build optimized production binary with Fat LTO
cargo build --release

# Optional: Add to system PATH or install locally
cargo install --path crates/nexuslb-cli
```

The compiled release binary is located at `./target/release/nexuslb`.

---

## 2. CLI Command Overview

NexusLB provides a comprehensive suite of CLI commands:

| Command | Usage | Description |
| :--- | :--- | :--- |
| `start` | `nexuslb start -c nexuslb.yaml` | Starts the load balancer with chosen worker threads and I/O engine. |
| `check` | `nexuslb check -c nexuslb.yaml` | Validates configuration syntax and backends without starting. |
| `reload` | `nexuslb reload [-a 127.0.0.1:9091]` | Signals a running NexusLB instance to atomically reload its configuration. |
| `status` | `nexuslb status [-a 127.0.0.1:9091]` | Queries active backends, latency metrics, and pool health. |
| `top` | `nexuslb top [-a http://127.0.0.1:9091]`| Launches the real-time terminal dashboard (TUI). |
| `benchmark` | `nexuslb benchmark system-info` | Displays system hardware, CPU cores, and build profile details. |
| `version` | `nexuslb version` | Prints version, target architecture, and active optimizations. |

---

## 3. Your First Load Balancer in 60 Seconds

Create a minimal configuration file named `nexuslb.yaml`:

```yaml
server:
  listen:
    - "0.0.0.0:8080"
  workers: "auto"
  engine: "tokio"

load_balancer:
  algorithm: "adaptive" # Or round_robin, least_conn, power_of_two_choices
  default_pool: "web"

backends:
  - name: "app-1"
    address: "127.0.0.1:3001"
    weight: 100
    protocol: "http1"
    pool: "web"

  - name: "app-2"
    address: "127.0.0.1:3002"
    weight: 100
    protocol: "http1"
    pool: "web"

health_check:
  enabled: true
  interval: "3s"
  timeout: "1s"
  http_path: "/health"
  expected_status: 200

admin:
  enabled: true
  address: "127.0.0.1:9091"

metrics:
  enabled: true
  address: "0.0.0.0:9090"
```

### Validate Configuration
```bash
./target/release/nexuslb check -c nexuslb.yaml
# Output: Configuration 'nexuslb.yaml' is valid.
```

### Start NexusLB
```bash
./target/release/nexuslb start -c nexuslb.yaml
```

---

## 4. Real-Time Operator Dashboard (TUI)

In a separate terminal, launch the interactive live dashboard:

```bash
./target/release/nexuslb top
```

You'll see real-time CPU thread activity, throughput (req/s), latency distributions (P50/P90/P99), backend health states, and error rates updated live at 10 Hz.

---

## 5. Zero-Downtime Hot Reload

When updating backends, routes, rate limits, or SSL certificates, you never need to stop NexusLB:

```bash
# Method 1: CLI Reload command via Admin REST API
./target/release/nexuslb reload -a 127.0.0.1:9091

# Method 2: HTTP POST directly to Admin API
curl -X POST http://127.0.0.1:9091/reload

# Method 3: Standard Unix SIGHUP signal (zero external network dependency)
kill -HUP $(pgrep nexuslb)
```

NexusLB parses and validates the new YAML in isolation. If valid, the new routing table and pools are swapped atomically via `ArcSwap` in **under 1 microsecond**. In-flight requests are never dropped.
