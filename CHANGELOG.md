# Changelog

All notable changes to NexusLB will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.2] - 2026-09-26 (Pre-Release)

### Added & Enhanced
- **Apple Human Interface UI Portal**:
  - Implemented sleek Apple SF-style icon toolbar navigation in the web portal and documentation center.
  - Expanded negative space, section breathing room (130px section padding), and responsive typography scale.
  - Replaced high-saturation neon gradients with Apple Titanium / Silver materials and subtle Apple Blue accents.
  - Added dedicated zero-telemetry Privacy Policy (`/privacy`), Security disclosure runbook (`/security`), and Terms of Service with attribution (`/terms`).
- **Domain & Deployment**:
  - Production documentation portal deployed to `https://nexuslb.arqora.work`.
- **Core Engine & Showdown**:
  - Bumped workspace dependencies to `0.0.2-pre`.
  - Comprehensive empirical showdown benchmarks vs NGINX, HAProxy, and Envoy across 50, 100, and 250 concurrency.

## [0.0.1] - 2026-09-26

### Added
- **Core Architecture**:
  - Workspace structure with 18 modular crates and clean abstraction boundaries.
  - Multi-engine architecture: Tokio (universal), Linux io_uring, and experimental AF_XDP.
  - Per-core worker model with `SO_REUSEPORT` kernel dispatch and CPU core affinity.
- **Dataplane**:
  - Lock-free buffer pool (`BufferPool`) eliminating per-request allocations (~13× faster than malloc).
  - Upstream connection pooling (`ConnectionPool`) with idle timeouts and health checks.
  - Layer 4 TCP proxying and Layer 7 HTTP/1.1 proxying with zero-allocation `httparse`.
  - WebSocket upgrade passthrough supporting transparent bidirectional streaming.
  - TLS termination powered by Rustls with dynamic SNI and zero-downtime certificate reloading.
- **Scheduling**:
  - 10 distinct load-balancing algorithms: Round Robin, Weighted Round Robin, Least Connections, Random, IP Hash, Consistent Hash (Ketama), Power of Two Choices (P2C), Least Latency, EWMA Latency, and Adaptive.
  - Adaptive composite scoring formula balancing EWMA latency, queue pressure, error rates, and circuit breaker state.
- **Reliability**:
  - Circuit breaker with `CLOSED`, `OPEN`, and `HALF_OPEN` states.
  - Active background health checks (TCP and HTTP GET probes) with threshold counters.
  - Passive health detection for connection resets, timeouts, and 5xx responses.
  - Graceful backend draining with configurable timeouts.
  - Idempotent request retries with exponential backoff and jitter.
  - Atomic token bucket rate limiter (global and per-client IP).
- **Observability**:
  - Prometheus metrics exporter (`/metrics` on port 9090) with cacheline-padded atomic counters.
  - Structured JSON and human-readable logging via `tracing`.
  - Admin REST API (`/health`, `/ready`, `/backends`, `/backends/:id/drain`, `/config`, `/reload`).
- **Benchmarking & Tooling**:
  - Comprehensive Criterion microbenchmarks for all schedulers, routing, and memory pools.
  - Standalone high-concurrency load testing harness (`nexuslb-bench`).
  - Hardware system information collector.
  - Performance baseline regression tracking (`performance-baseline.json`).
- **Deployments**:
  - Multi-stage minimal Dockerfile and Docker Compose demo.
  - Production Kubernetes manifests (Deployment, Service, ConfigMap).
  - Hardened Systemd unit file with security sandboxing and resource limits.
