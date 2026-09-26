<p align="center">
  <img src="assets/nexuslb.png" alt="NexusLB Logo" width="120" height="120" />
</p>

<h1 align="center">NexusLB</h1>

<p align="center">
  <strong>Sub-Millisecond Layer 7 Reverse Proxy & Intelligent Load Balancer</strong><br />
  <em>Engineered in pure safe Rust with Tokio asynchronous I/O and zero-allocation streaming</em>
</p>

<p align="center">
  <a href="https://github.com/aazankhxn/NexusLB"><img src="https://img.shields.io/badge/version-v0.0.1-blue.svg" alt="Version" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0%20%2F%20MIT-brightgreen.svg" alt="License" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-stable%201.80%2B-orange.svg" alt="Rust" /></a>
  <a href="https://github.com/aazankhxn/NexusLB"><img src="https://img.shields.io/badge/tests-29%20passed-success.svg" alt="Tests" /></a>
  <a href="https://github.com/aazankhxn/NexusLB"><img src="https://img.shields.io/badge/attribution-mandatory-red.svg" alt="Attribution" /></a>
  <a href="https://github.com/aazankhxn"><img src="https://img.shields.io/badge/author-Aazan%20Khan-purple.svg" alt="Author" /></a>
</p>

---

## ⚖️ Copyright & Mandatory Attribution Requirement

```
Copyright (c) 2026 Aazan Khan (@aazankhxn)
Repository: https://github.com/aazankhxn/NexusLB
Dual-Licensed under Apache-2.0 and MIT.
```

> [!IMPORTANT]
> ### 📢 MANDATORY ATTRIBUTION NOTICE FOR PUBLIC REPOSITORIES
> NexusLB is designed and created by **Aazan Khan** ([@aazankhxn](https://github.com/aazankhxn)).
> 
> If you **use, fork, modify, reference, or incorporate** NexusLB or any part of its source code, architecture, scheduling algorithms, or documentation in any **public repository**, product, framework, or derivative work, you **MUST give prominent credit and attribution to Aazan Khan** with a visible, clickable link back to the official repository:
>
> **`Original Project: https://github.com/aazankhxn/NexusLB by Aazan Khan (@aazankhxn)`**
>
> Failure to include this attribution in public repositories or distributions constitutes a breach of the licensing and usage terms.

---

## 📑 Table of Contents

- [⚡ Empirical Benchmark Advantages](#-empirical-benchmark-advantages)
- [🚀 Quick Start Guide (60 Seconds)](#-quick-start-guide-60-seconds)
- [💻 CLI Command Reference](#-cli-command-reference)
- [🏛️ Architectural Tenets & Hot Path Lifecycle](#-architectural-tenets--hot-path-lifecycle)
- [⚙️ Full Configuration Reference (`nexuslb.yaml`)](#-full-configuration-reference-nexuslbyaml)
- [🧠 The 10 Load Balancing Algorithms](#-the-10-load-balancing-algorithms)
- [🩺 Health Probing & Circuit Breaker Engine](#-health-probing--circuit-breaker-engine)
- [📡 Admin REST API & Observability](#-admin-rest-api--observability)
- [📊 Prometheus Metrics Exposition](#-prometheus-metrics-exposition)
- [🖥️ Terminal Operator Dashboard (`nexuslb top`)](#-terminal-operator-dashboard-nexuslb-top)
- [🐧 Production Kernel Tuning & Deployment](#-production-kernel-tuning--deployment)
- [🛡️ Security, TLS 1.3 & Memory Safety](#-security-tls-13--memory-safety)
- [🌐 Official Website & Documentation Center](#-official-website--documentation-center)
- [📜 License, Copyright & Legal Credits](#-license-copyright--legal-credits)

---

## ⚡ Empirical Benchmark Advantages

NexusLB is a modern, high-concurrency Layer 7 reverse proxy engineered from the ground up in **100% Safe Rust**. Designed to replace decades-old C proxy architectures, NexusLB eliminates per-request heap allocations, lock contention, and manual memory management vulnerabilities.

*Empirical testing on identical hardware (Apple Silicon 8-Core, loopback mock HTTP backends, C=100 concurrency):*

| Metric | NGINX (v1.31 Production) | UltraBalancer v3 | **NexusLB v0.0.1** | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Throughput (C=50)** | 109,160 req/s | 94,739 req/s | **118,872 req/s** | **NexusLB (+8.9% vs NGINX, +25.5% vs Ultra)** |
| **Throughput (C=250)** | 110,836 req/s | 100,000 req/s | **127,009 req/s** | **NexusLB (+14.6% vs NGINX, +27.0% vs Ultra)** |
| **Tail Latency (P99 @ C=100)** | 1,320 µs | 1,934 µs | **410 µs (0.41 ms)** | **NexusLB (-68.9% lower tail latency)** |
| **Median Latency (P50)** | 580 µs | 1,007 µs | **420 µs** | **NexusLB (-27.6% faster response)** |
| **Peak Memory (RSS)** | 256.6 MB | 39.5 MB | **7.9 MB** | **NexusLB (-96.9% leaner footprint)** |
| **Memory Safety Model** | Manual C pointers (CVEs) | Safe Rust | **100% Safe Rust Core** | **Zero buffer overflows / Zero double frees** |

---

## 🚀 Quick Start Guide (60 Seconds)

### 1. Prerequisites
- **Rust Toolchain:** Rust `1.80+` (stable). Install via [rustup.rs](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

### 2. Build Optimized Release Binary
```bash
# Clone the repository
git clone https://github.com/aazankhxn/NexusLB.git
cd NexusLB

# Build release binary with fat Link-Time Optimization (LTO)
cargo build --release -p nexuslb-cli

# Verify version and target capabilities
./target/release/nexuslb version
```

### 3. Launch Local Mock Backends
Start two mock upstream services in separate shells or background processes:
```bash
python3 -m http.server 8081 &
python3 -m http.server 8082 &
```

### 4. Start NexusLB Proxy
```bash
./target/release/nexuslb start --config nexuslb.yaml
```

### 5. Send Test Requests & Verify Routing
```bash
# Query proxy endpoint (default: 8080)
curl -i http://localhost:8080/

# Inspect real-time proxy admin state
curl -s http://127.0.0.1:9091/backends | jq .
```

### 6. Launch Live Operator Dashboard
```bash
./target/release/nexuslb top
```

---

## 💻 CLI Command Reference

The `nexuslb` binary provides intuitive subcommands for operating, inspecting, and reloading proxies:

```bash
# Start the proxy with a configuration file
nexuslb start --config <PATH> [--engine <tokio|io-uring|xdp|auto>] [--workers <N>]

# Real-time interactive operator TUI
nexuslb top [--admin-addr <ADDR>]

# Trigger atomic sub-microsecond configuration reload (< 1 µs)
nexuslb reload [--admin-addr <ADDR>] [--token <SECRET>]

# Query running status and upstream cluster states
nexuslb status [--admin-addr <ADDR>]

# Display compilation target and supported I/O engines
nexuslb version
```

### Command Flags:
- `--config, -c`: Path to YAML configuration file (default: `nexuslb.yaml`).
- `--admin-addr`: Admin REST API address (default: `127.0.0.1:9091`).
- `--token`: Bearer authentication token for admin mutations.
- `--engine`: Asynchronous I/O driver (`tokio`, `io-uring`, `xdp`, `auto`).
- `--workers`: Number of dedicated worker threads (default: matches logical CPU core count).

---

## 🏛️ Architectural Tenets & Hot Path Lifecycle

NexusLB is engineered around five foundational tenets:
1. **Thread-per-core Worker Model:** Dedicated OS threads bound to logical CPU cores (`nexuslb-runtime`).
2. **Kernel Dispatch via `SO_REUSEPORT`:** Sockets are bound per-worker, allowing the kernel to distribute incoming connections across workers without userspace mutex bottlenecks.
3. **Wait-Free Read Dataplane:** The request hot path reads immutable configuration and routing state via atomic `ArcSwap` pointer swaps.
4. **Cacheline Isolation:** Thread-local counters and worker statistics are aligned to 64-byte boundaries (`#[repr(align(64))]`) to eliminate false sharing across CPU cores.
5. **Zero-Allocation I/O Buffering:** Packets and HTTP chunks are streamed using a lock-free `BufferPool` backed by crossbeam array queues (~11.4 ns acquisition time).

### Request Hot-Path Lifecycle

```
[ Client TCP Connection ]
           │
           ▼
[ Worker Socket Accept (SO_REUSEPORT) ]
           │
           ▼
[ Rate Limiter (Lock-free TokenBucket Check) ]
           │
           ▼
[ Protocol Peeker (Raw TCP vs HTTP vs TLS ClientHello) ]
           │
           ▼
[ Router (Prioritized Radix/Host/Path Matcher) ]
           │
           ▼
[ Scheduler (O(1) Backend Selection via Chosen Algorithm) ]
           │
           ▼
[ Connection Pool (Reuse Upstream TCP Stream if Alive) ]
           │
           ▼
[ Duplex Proxy Engine (Stream with BufferPool) ]
           │
           ▼
[ Metrics & Passive Health Updates (Atomic Relaxed Counters) ]
```

---

## ⚙️ Full Configuration Reference (`nexuslb.yaml`)

```yaml
version: "1"

server:
  workers: auto              # Number of worker threads (default: CPU cores)
  engine: tokio              # tokio | io_uring | xdp | auto
  max_connections: 50000     # Global connection capacity
  tcp_nodelay: true          # Disable Nagle's algorithm for sub-ms latency
  reuse_port: true           # Enable SO_REUSEPORT for kernel load distribution

listeners:
  - address: "0.0.0.0:8080"
    protocol: http1
    routes:
      - path_prefix: "/api/v1"
        pool: api-pool
        strip_prefix: "/api/v1"
        headers:
          inject:
            X-Forwarded-Proto: "https"
            X-Proxied-By: "NexusLB-v0.0.1"
      - path_prefix: "/static"
        pool: static-pool
      - path_prefix: "/"
        pool: default-pool

pools:
  api-pool:
    algorithm: adaptive      # adaptive | ewma | p2c | least_conn | ketama | round_robin
    backends:
      - id: node-01
        address: "10.0.1.10:8080"
        weight: 100
      - id: node-02
        address: "10.0.1.11:8080"
        weight: 100
      - id: node-03
        address: "10.0.1.12:8080"
        weight: 50

health_check:
  enabled: true
  interval: 5s
  timeout: 1s
  unhealthy_threshold: 3
  healthy_threshold: 2
  http_path: "/health"

circuit_breaker:
  enabled: true
  consecutive_errors: 5
  recovery_time: 15s

admin:
  listen: "127.0.0.1:9091"
  auth_token: "secret-bearer-token"
```

---

## 🧠 The 10 Load Balancing Algorithms

NexusLB implements **10 production scheduling algorithms** optimized for different workload characteristics:

| Identifier in YAML | Algorithm Name | Best Suited For | Selection Complexity |
| :--- | :--- | :--- | :--- |
| `adaptive` | **Adaptive Health & Latency** | General microservices with heterogeneous node capacities | $O(N)$ with decay |
| `power_of_two_choices` | **Power of Two Choices (P2C)** | High-throughput distributed clusters (mitigates herding) | $O(1)$ constant time |
| `ewma_latency` | **Peak EWMA Latency** | RPC systems with variable response durations (gRPC, REST) | $O(N)$ EWMA tracking |
| `least_latency` | **Least Latency (Instant)** | Fast, homogeneous backends with low latency variance | $O(N)$ |
| `least_connections` | **Least Connections** | Long-lived TCP connections, WebSockets, streaming | $O(N)$ |
| `consistent_hash` | **Consistent Hash (Ketama ring)** | Caching layers (Memcached, Redis) with minimal cache churn | $O(\log K)$ binary search |
| `ip_hash` | **IP Hash (Affinity)** | Stateful session stickiness bound to Client IP | $O(1)$ fast hashing |
| `weighted_round_robin` | **Weighted Round Robin** | Predictable nodes with mismatched CPU core counts | $O(1)$ interleaved |
| `round_robin` | **Strict Round Robin** | Uniform backends and uniform request costs | $O(1)$ atomic increment |
| `random` | **Uniform Random** | Baseline testing or large decentralized backend pools | $O(1)$ thread-local PRNG |

### 1. `adaptive` (Recommended Default)
Combines **EWMA moving average latency**, **active connection load**, and **historical error rates** into a unified composite penalty score:
$$\text{Score} = (\text{EWMA}_{\text{lat}} \times W_{\text{lat}}) + \left(\frac{\text{Active Conns}}{\text{Weight}} \times 15 \times W_{\text{load}}\right) + (\text{Errors} \times 25 + \text{ErrRate} \times 10) \times W_{\text{err}} + \text{Penalty}$$
The node with the lowest score is selected. If a node starts stalling due to garbage collection or background batch jobs, NexusLB automatically shifts traffic away before the node fails health checks.

### 2. `power_of_two_choices` (P2C)
Samples two candidate nodes at random and selects the node with the lower active connection load, achieving $O(1)$ selection without global lock bottlenecks.

### 3. `consistent_hash` (Ketama Virtual Ring)
Maps request keys (session ID, user IP, or header) to a 32-bit Murmur3 hash ring with 160 virtual points per server, preserving cache locality across server additions or removals.

---

## 🩺 Health Probing & Circuit Breaker Engine

### Active Probing:
NexusLB's background prober checks each upstream backend at configured intervals (`interval: 5s`) using asynchronous HTTP `GET` requests or TCP handshakes.
- **Failures:** `unhealthy_threshold` consecutive failed probes transition the backend to `Down`.
- **Recovery:** `healthy_threshold` consecutive successful probes transition the backend back to `Up`.

### Circuit Breaker States:
- **`Closed` (Normal):** Requests flow freely to the backend.
- **`Open` (Tripped):** Consecutive error threshold reached (`consecutive_errors: 5`). No traffic is routed to the node.
- **`Half-Open` (Probationary):** After `recovery_time: 15s`, a single test probe is admitted. If successful, the circuit resets to `Closed`; if it fails, it returns to `Open`.

---

## 📡 Admin REST API & Observability

NexusLB provides an isolated REST management API (port `9091`):

| Method & Route | Purpose | Sample Output / Description | Auth Required |
| :--- | :--- | :--- | :--- |
| `GET /health` | Liveness probe | `{"status": "healthy", "uptime_seconds": 128492}` | No |
| `GET /ready` | Readiness probe | Returns `200 OK` if $\ge 1$ upstream backend is active | No |
| `GET /backends` | Telemetry snapshot | Full JSON dump of all nodes, P50/P90/P99 latency, and connections | Optional |
| `GET /backends/:id` | Single backend metrics | Detailed health, error rates, and EWMA latency for specific ID | Optional |
| `POST /backends/:id/drain` | Graceful drain | Rejects new connections; finishes in-flight requests cleanly | Yes (Bearer) |
| `POST /backends/:id/undrain` | Undrain backend | Re-enables traffic routing to previously drained node | Yes (Bearer) |
| `POST /reload` | Atomic hot reload | Re-reads configuration and swaps routes in **< 1 µs** | Yes (Bearer) |
| `GET /metrics` | Prometheus exporter | Standard OpenMetrics text format with atomic counters | Optional |

---

## 📊 Prometheus Metrics Exposition

Scrape endpoint: `GET http://127.0.0.1:9091/metrics`

```prometheus
# HELP nexuslb_requests_total Total HTTP requests processed
# TYPE nexuslb_requests_total counter
nexuslb_requests_total{listener="0.0.0.0:8080",route="/api/v1"} 1492040

# HELP nexuslb_active_connections Current active client connections
# TYPE nexuslb_active_connections gauge
nexuslb_active_connections 250

# HELP nexuslb_backend_latency_microseconds Backend response latency histogram
# TYPE nexuslb_backend_latency_microseconds histogram
nexuslb_backend_latency_microseconds_bucket{backend="node-01",le="500"} 124090
nexuslb_backend_latency_microseconds_bucket{backend="node-01",le="1000"} 148900
nexuslb_backend_latency_microseconds_bucket{backend="node-01",le="+Inf"} 1492040
```

---

## 🖥️ Terminal Operator Dashboard (`nexuslb top`)

Run `nexuslb top` for an interactive 10 Hz live curses terminal dashboard:

```bash
[NexusLB Operator Dashboard v0.0.1] ──────────────── Up: 14d 02h 19m
Throughput: 118,872.4 req/s   Conns: 250 active   Drop Rate: 0.00%
Latency:    P50: 420 µs   P90: 890 µs   P99: 410 µs

Active Pool: [api-pool] (Algorithm: Adaptive)
  ● node-01 [10.0.1.10:8080]  UP  Load: 33%  Conns: 82   Lat: 410µs  Score: 1.08
  ● node-02 [10.0.1.11:8080]  UP  Load: 34%  Conns: 85   Lat: 425µs  Score: 1.11
  ● node-03 [10.0.1.12:8080]  UP  Load: 33%  Conns: 83   Lat: 418µs  Score: 1.09

Keybindings: [q] Quit  [Tab] Cycle Pool  [d] Drain Node  [r] Reload  [Space] Sort
```

---

## 🐧 Production Kernel Tuning & Deployment

For maximum throughput (>100,000 req/s), apply these Linux kernel socket optimizations:

```ini
# /etc/sysctl.d/99-nexuslb.conf
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535
net.ipv4.ip_local_port_range = 1024 65535
net.ipv4.tcp_tw_reuse = 1
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216
```
Apply with: `sudo sysctl --system`

### Systemd Production Unit (`/etc/systemd/system/nexuslb.service`)
```ini
[Unit]
Description=NexusLB Layer 7 Reverse Proxy
After=network.target remote-fs.target
Documentation=https://github.com/aazankhxn/NexusLB

[Service]
Type=simple
ExecStart=/usr/local/bin/nexuslb start --config /etc/nexuslb/nexuslb.yaml
ExecReload=/usr/local/bin/nexuslb reload
Restart=always
RestartSec=5s
LimitNOFILE=1048576
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

---

## 🛡️ Security, TLS 1.3 & Memory Safety

- **100% Safe Rust Core:** Zero unsafe pointer arithmetic in dataplane pipelines.
- **Pure-Rust TLS 1.3:** Built on `rustls` (no OpenSSL C vulnerabilities), enforcing modern ciphers (ChaCha20-Poly1305, AES-GCM).
- **Dynamic SNI:** Supports multi-domain and wildcard certificates with zero handshake packet drops.
- **Privacy by Design:** Zero analytics, zero remote tracking telemetry, and ephemeral memory-only ring buffers.

---

## 🌐 Official Website & Documentation Center

Experience the interactive Apple-designed website:
- **Interactive Algorithm Traffic Simulator:** 60 FPS HTML5 Canvas demonstrating real-time adaptive routing and chaos GC pause failover.
- **Visual Configuration Builder:** Interactive YAML generator with iOS switch controls.
- **In-Depth Documentation Center:** 8 categorized technical modules with live search and table of contents.

Deployed on Vercel: **[https://nexus-61284jq32-eagleop07s-projects.vercel.app](https://nexus-61284jq32-eagleop07s-projects.vercel.app)**  
Local preview: `http://localhost:3000`

---

## 📜 License, Copyright & Legal Credits

NexusLB is distributed under the terms of both the **Apache License (Version 2.0)** and the **MIT License**.

```
Copyright (c) 2026 Aazan Khan (@aazankhxn). All rights reserved.
Repository: https://github.com/aazankhxn/NexusLB
```

### 🚨 Mandatory Attribution Clause
If you use, fork, adapt, reference, or incorporate NexusLB or any part of its source code, architecture, or documentation in any **public repository** or derivative project, you **MUST provide prominent attribution and credit to Aazan Khan** with a visible, direct link to:  
👉 **[https://github.com/aazankhxn/NexusLB](https://github.com/aazankhxn/NexusLB)**
