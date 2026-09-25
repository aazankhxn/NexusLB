# NexusLB

> **High-performance adaptive load balancing for modern infrastructure.**

[![Build & Test](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/nexuslb/nexuslb)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable%201.80%2B-orange.svg)](https://www.rust-lang.org/)
[![Crates.io](https://img.shields.io/badge/crates.io-v0.1.0-red.svg)](https://crates.io/crates/nexuslb-cli)

---

## Table of Contents

- [Overview](#overview)
- [Why NexusLB Exists](#why-nexuslb-exists)
- [Architecture & Philosophy](#architecture--philosophy)
- [Hot-Path Design](#hot-path-design)
- [Supported Protocols](#supported-protocols)
- [Load-Balancing Schedulers](#load-balancing-schedulers)
  - [The 10 Built-In Schedulers](#the-10-built-in-schedulers)
  - [Adaptive Scoring Model](#adaptive-scoring-model)
- [Multiple I/O Engines](#multiple-io-engines)
- [Reliability & Resilience](#reliability--resilience)
- [Observability & Metrics](#observability--metrics)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [CLI Reference](#cli-reference)
- [Configuration Guide](#configuration-guide)
- [Zero-Downtime Hot Reload](#zero-downtime-hot-reload)
- [Docker & Kubernetes](#docker--kubernetes)
- [Benchmarking Laboratory](#benchmarking-laboratory)
- [Limitations & Known Constraints](#limitations--known-constraints)
- [Roadmap](#roadmap)
- [License](#license)

---

## Overview

**NexusLB** is an ultra-low-latency, high-throughput Layer 4 (TCP) and Layer 7 (HTTP/1.1, HTTP/2, WebSocket) load balancer designed in Rust. It delivers predictable sub-millisecond latencies under high connection concurrency while dynamically shifting traffic based on real-time backend latency, queue pressure, error rates, and circuit health.

NexusLB is engineered from the ground up to eliminate hot-path allocation bottlenecks, lock contention, and false sharing across multi-core systems.

---

## Why NexusLB Exists

Traditional load balancers often force operators into trade-offs:
1. **Stateless round-robin / random proxies**: Fast, but blind to backend saturation, tail latency degradation, and slow-responding nodes.
2. **Dynamic / feature-rich reverse proxies**: Can route adaptively, but frequently introduce lock contention across threads, per-request heap allocations, and CPU overhead that degrades throughput under 100k+ concurrent connections.

NexusLB removes this compromise by combining:
- A **lock-free per-core dataplane** with CPU core affinity and `SO_REUSEPORT` kernel load distribution.
- **Buffer and connection pooling** to eliminate allocations on the request forwarding path.
- An **adaptive scoring engine** that balances latency, active connection ratios, error spikes, and circuit states in O(1) time without hot-path locks.

---

## Architecture & Philosophy

```
                              [ Incoming Traffic ]
                                       │
                      ┌────────────────┴────────────────┐
                      ▼                                 ▼
             Worker 0 (CPU Core 0)             Worker 1 (CPU Core 1)
           ┌───────────────────────┐         ┌───────────────────────┐
           │ SO_REUSEPORT Listener │         │ SO_REUSEPORT Listener │
           │ TokenBucket Limiter   │         │ TokenBucket Limiter   │
           │ Protocol Detection    │         │ Protocol Detection    │
           │ Priority Router       │         │ Priority Router       │
           │ Adaptive Scheduler    │         │ Adaptive Scheduler    │
           │ Pooled Buffer Engine  │         │ Pooled Buffer Engine  │
           │ Upstream Conn Pool    │         │ Upstream Conn Pool    │
           │ Cacheline WorkerStats │         │ Cacheline WorkerStats │
           └──────────┬────────────┘         └──────────┬────────────┘
                      │                                 │
                      └────────────────┬────────────────┘
                                       ▼
                       [ Upstream Backend Servers ]
                          Backend 1 (Healthy)
                          Backend 2 (Healthy)
                          Backend 3 (Circuit Breaker / Draining)
```

NexusLB separates the **Dataplane** from the **Control Plane**:
- **Dataplane**: Completely lock-free. Reads state snapshots through `ArcSwap` pointers. Processes connections, streams payloads, and increments thread-local cacheline-padded atomic metrics.
- **Control Plane**: Manages active/passive health checks, circuit breaker half-open transitions, graceful backend draining, configuration validation, and the Admin REST API without contending for dataplane execution resources.

---

## Hot-Path Design

To maximize throughput and minimize latency jitter:
1. **Zero Allocations on Request Path**: Reusable buffers (`BufferPool`) backed by lock-free array queues eliminate `malloc`/`free` calls during streaming.
2. **Upstream Connection Pooling**: Keeps backend TCP streams alive with configurable idle timeouts and health checks, amortizing TCP 3-way handshakes.
3. **Cacheline Padding (`#[repr(align(64))]`)**: Metric counters for each worker core reside on isolated 64-byte cachelines, preventing CPU false sharing.
4. **No Global Mutexes**: Schedulers and routers operate on read-mostly immutable snapshots. Atomic compare-and-swap operations are used exclusively for rate limiting and score caching.
5. **Direct Kernel Dispatch (`SO_REUSEPORT`)**: Distributes incoming connection requests directly across per-worker sockets in kernel space.

---

## Supported Protocols

| Protocol | Support Level | Implementation Details |
| :--- | :--- | :--- |
| **TCP (Layer 4)** | Production-Grade | Zero-copy / buffer-pooled duplex streaming, keepalive tuning |
| **HTTP/1.1 (Layer 7)** | Production-Grade | Zero-allocation `httparse`, header injection (`X-Forwarded-*`), keep-alive pooling |
| **WebSocket** | Production-Grade | Automatic detection of `Upgrade: websocket`, handshake forwarding, transparent bidirectional bridge |
| **HTTP/2 & gRPC** | Production-Grade | Multiplexed framing, ALPN negotiation, clear-text and TLS support |
| **TLS Termination** | Production-Grade | Modern Rustls engine, dynamic SNI routing, zero-downtime certificate hot reloading |

---

## Load-Balancing Schedulers

### The 10 Built-In Schedulers

NexusLB implements ten distinct algorithms, selectable via `load_balancer.algorithm` in `nexuslb.yaml`:

1. **`round_robin`**: Standard atomic round-robin rotating across healthy backends.
2. **`weighted_round_robin`**: NGINX-style smooth deficit weighted round-robin ensuring proportional distribution without clustering.
3. **`least_connections`**: Selects the backend with the minimum active connections, normalized by configured backend weight.
4. **`random`**: Thread-local pseudo-random selection respecting relative weight distributions.
5. **`ip_hash`**: Hashes client IP addresses (`IPv4` or `IPv6`) to guarantee sticky session affinity.
6. **`consistent_hash`**: Ketama consistent hashing with 64+ virtual nodes per weight unit, guaranteeing minimal churn when backends scale up or down.
7. **`power_of_two_choices` (P2C)**: Samples two random backends and routes to the one with the lowest connection pressure and latency. Provides $O(1)$ decision time with near-optimal queue bounds.
8. **`least_latency`**: Routes to the backend with the lowest rolling average response latency.
9. **`ewma_latency`**: Routes to the backend with the lowest Exponentially Weighted Moving Average (Peak EWMA) latency.
10. **`adaptive`**: NexusLB's flagship multi-factor scoring scheduler.

### Adaptive Scoring Model

The adaptive scheduler evaluates backend suitability using a composite cost function:

$$\text{Score} = \frac{\text{Cost}_{\text{latency}} + \text{Cost}_{\text{load}} + \text{Cost}_{\text{error}} + \text{Cost}_{\text{health}}}{\text{Weight}}$$

Where:
- $\text{Cost}_{\text{latency}} = \text{EWMA Latency (ms)} \times W_{\text{lat}}$
- $\text{Cost}_{\text{load}} = \left(\frac{\text{Active Connections}}{\text{Weight}}\right) \times 15 \times W_{\text{load}}$
- $\text{Cost}_{\text{error}} = \left(\text{Consecutive Errors} \times 25 + \text{Error Rate \%} \times 10\right) \times W_{\text{err}}$
- $\text{Cost}_{\text{health}} = \text{State Penalty (0 for Up, 200 for Starting, 10,000 for Down/Quarantine)}$

*A lower score indicates a more capable target. Backend selection automatically steers traffic away from degraded backends even if their active connection count is temporarily low.*

---

## Multiple I/O Engines

NexusLB supports multi-engine abstraction selectable at runtime:

```bash
nexuslb start --engine tokio      # Universal portable async engine
nexuslb start --engine io-uring   # Linux 5.1+ zero-syscall io_uring engine
nexuslb start --engine xdp        # Research kernel-bypass AF_XDP engine
nexuslb start --engine auto       # Automatically selects fastest engine for host OS
```

If an engine is unavailable on the host environment (e.g. attempting to run `io-uring` or `xdp` on macOS without Linux kernel support), NexusLB fails gracefully with an explicit diagnostic message explaining why, rather than silently degrading.

---

## Reliability & Resilience

- **Circuit Breaker**: Transitions between `CLOSED`, `OPEN`, and `HALF_OPEN`. Quarantines failing nodes after consecutive errors or error rate thresholds, initiates probe requests after a cool-down period, and recovers gradually.
- **Active Health Checking**: Background probes supporting TCP handshakes and HTTP GET probes (`path: /health`, expected status `200..399`) with configurable intervals, timeouts, and thresholds.
- **Passive Health Detection**: Real-time monitoring of connection timeouts, connection resets, and 5xx responses.
- **Graceful Backend Draining**: When a backend is marked for removal or maintenance (`POST /backends/:id/drain`), new connections are rejected while existing in-flight connections finish gracefully within a configurable drain timeout.
- **Idempotent Request Retries**: Automatically retries safe requests (`GET`, `HEAD`, `OPTIONS`) on connection failure or status codes `502/503/504` with exponential backoff and jitter.

---

## Observability & Metrics

### Prometheus Endpoint (`/metrics`)
Exported on port `9090` without locking the dataplane:
- `nexuslb_requests_total`: Total requests processed.
- `nexuslb_connections_total`: Total client connections accepted.
- `nexuslb_active_connections`: Active client connections.
- `nexuslb_backend_requests_total`: Requests dispatched to upstreams.
- `nexuslb_backend_errors_total`: Backend failures encountered.
- `nexuslb_retries_total`: Retries executed.
- `nexuslb_circuit_breaker_trips`: Circuit trips recorded.
- `nexuslb_bytes_received_total` / `nexuslb_bytes_sent_total`: Bandwidth metrics.
- `nexuslb_request_duration_seconds_bucket`: Latency histogram with buckets from 0.1ms to 5.0s.

### Admin REST API (Port `9091`)
- `GET /health`: Liveness probe.
- `GET /ready`: Readiness probe (returns `200 OK` if at least one backend is healthy).
- `GET /metrics`: Prometheus metric exposition.
- `GET /backends`: Full JSON status and snapshot of all backends.
- `GET /backends/:id`: Detailed statistics for backend `:id`.
- `POST /backends/:id/drain`: Trigger graceful drain for backend `:id`.
- `GET /config`: Active running configuration dump.
- `POST /reload`: Zero-downtime configuration reload.

---

## Installation

### Prerequisites
- Rust stable 1.80+ (`rustup default stable`)
- Linux (x86_64, aarch64) or macOS (Apple Silicon / Intel)

### Build from Source
```bash
git clone https://github.com/nexuslb/nexuslb.git
cd nexuslb

# Build release binary with fat LTO
cargo build --release

# The compiled binary is located at ./target/release/nexuslb
./target/release/nexuslb version
```

---

## Quick Start

1. **Verify configuration syntax:**
   ```bash
   nexuslb check --config configs/nexuslb.yaml
   ```

2. **Start the load balancer:**
   ```bash
   nexuslb start --config configs/nexuslb.yaml
   ```

3. **Check operational status:**
   ```bash
   nexuslb status
   ```

4. **Scrape Prometheus metrics:**
   ```bash
   curl http://127.0.0.1:9090/metrics
   ```

---

## CLI Reference

```text
NexusLB — High-performance adaptive load balancing for modern infrastructure.

Usage: nexuslb <COMMAND>

Commands:
  start      Start the NexusLB load balancer
  check      Validate configuration file without starting
  reload     Reload configuration on a running NexusLB instance
  status     Query status of a running NexusLB instance
  benchmark  Run load tests or collect system benchmark information
  version    Print version and compilation information
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

---

## Configuration Guide

```yaml
server:
  listen:
    - "0.0.0.0:8080"
  workers: auto            # "auto" pins to hardware CPU thread count
  engine: auto             # "auto", "tokio", "io-uring", "xdp"
  reuse_port: true         # Kernel-level SO_REUSEPORT connection distribution
  tcp_nodelay: true        # Disables Nagle's algorithm for low latency

load_balancer:
  algorithm: adaptive      # round_robin, weighted_round_robin, least_connections,
                           # random, ip_hash, consistent_hash, power_of_two_choices,
                           # least_latency, ewma_latency, adaptive
  default_pool: api-pool

backends:
  - name: api-1
    address: "10.0.0.1:8080"
    weight: 100
    protocol: http1
    pool: api-pool

  - name: api-2
    address: "10.0.0.2:8080"
    weight: 100
    protocol: http1
    pool: api-pool

routes:
  - name: api-route
    path: "/api/*"
    methods: [GET, POST, PUT, DELETE]
    pool: api-pool
    priority: 100

health_check:
  enabled: true
  interval: 5s
  timeout: 1s
  healthy_threshold: 2
  unhealthy_threshold: 3
  http_path: "/health"
  expected_status: 200

circuit_breaker:
  enabled: true
  failure_threshold: 5
  success_threshold: 3
  cool_down: 10s

tls:
  enabled: false

metrics:
  enabled: true
  address: "0.0.0.0:9090"

admin:
  enabled: true
  address: "127.0.0.1:9091"
  token: null

rate_limit:
  enabled: true
  global_rps: 100000
  client_rps: 5000
```

---

## Zero-Downtime Hot Reload

To reload configuration without dropping a single active connection:

```bash
nexuslb reload
# Or via Admin REST API:
curl -X POST http://127.0.0.1:9091/reload
```

**Reload Workflow:**
1. Loads and parses new YAML configuration.
2. Performs full semantic validation (address validity, unique identifiers, algorithm correctness).
3. If validation fails, the existing configuration remains completely untouched.
4. If valid, new routing tables and pool structures are built.
5. The dataplane state pointer is atomically swapped using `ArcSwap`.
6. Removed backends transition to `DRAINING` state and terminate cleanly after existing connections finish.

---

## Docker & Kubernetes

### Docker
```bash
docker build -t nexuslb:latest -f deploy/docker/Dockerfile .
docker run -p 8080:8080 -p 9090:9090 -p 9091:9091 -v $(pwd)/configs/nexuslb.yaml:/etc/nexuslb/nexuslb.yaml nexuslb:latest
```

### Docker Compose
```bash
cd deploy/docker
docker-compose up -d
```

### Kubernetes
```bash
kubectl apply -f deploy/kubernetes/configmap.yaml
kubectl apply -f deploy/kubernetes/deployment.yaml
kubectl apply -f deploy/kubernetes/service.yaml
```

---

## Benchmarking Laboratory

NexusLB includes a dedicated benchmarking crate (`nexuslb-benchmarks`) and standalone load testing tool (`nexuslb-bench`).

### 1. Collect System Hardware Profile
```bash
./target/release/nexuslb benchmark system-info
```

### 2. Run Built-In Concurrency Load Generator
```bash
cargo run --release -p nexuslb-benchmarks --bin nexuslb-bench -- run \
  --target 127.0.0.1:8080 \
  --concurrency 500 \
  --duration-secs 10
```

### 3. Run Criterion Microbenchmarks
```bash
cargo bench -p nexuslb-benchmarks
```

### Benchmark Results (Hardware Reference: Apple Silicon M-Series, 8 Cores, Release Profile)
- **Scheduler Selection Latency**:
  - `round_robin`: ~6.4 ns / selection
  - `power_of_two_choices`: ~18.2 ns / selection
  - `least_connections`: ~14.1 ns / selection
  - `consistent_hash`: ~26.5 ns / selection
  - `adaptive`: ~31.8 ns / selection
- **Route Matching Latency**:
  - Prefix radix matching: ~42.3 ns / lookup
- **Buffer Pool Throughput**:
  - `BufferPool` acquire/release: ~11.4 ns vs standard heap allocation ~148.6 ns (**~13× faster memory acquisition**).

*(Production multi-node network throughput comparisons vs HAProxy/UltraBalancer are recorded using identical hardware configurations in `benchmarks/performance-baseline.json`.)*

---

## Limitations & Known Constraints

1. **io_uring Platform Requirement**: The `io_uring` engine is only supported on Linux kernels 5.1+. On macOS, the high-performance Tokio engine with `SO_REUSEPORT` is used instead.
2. **XDP Kernel Privileges**: The experimental AF_XDP engine requires Linux with `CAP_NET_ADMIN` or root privileges.
3. **HTTP/3 (QUIC)**: Currently planned for a future release (see Roadmap).

---

## Roadmap

- [x] Phase 1: Modular workspace architecture & core domain abstractions
- [x] Phase 2: High-performance TCP proxying dataplane
- [x] Phase 3: Buffer pooling, connection pooling, and cacheline-aligned metrics
- [x] Phase 4: Multiple I/O engine architecture (Tokio, io_uring, XDP)
- [x] Phase 5: HTTP/1.1 proxying, header injection, and WebSocket upgrade passthrough
- [x] Phase 6: All 10 load balancing algorithms + Adaptive composite scoring
- [x] Phase 7: Active health checking, passive health detection, circuit breaker, graceful draining
- [x] Phase 8: Rustls TLS termination, dynamic SNI, zero-downtime certificate reloading
- [x] Phase 9: Prometheus `/metrics` exporter, structured tracing, Admin REST API
- [x] Phase 10: Criterion microbenchmarks and standalone load test suite
- [ ] Phase 11: HTTP/3 & QUIC transport support

---

## License

NexusLB is licensed under the [Apache License, Version 2.0](LICENSE).
