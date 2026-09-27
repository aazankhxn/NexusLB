# Changelog

All notable changes to NexusLB will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.4] - 2026-09-27

### Security, Resilience & Telemetry Hardening
- **Zero-Bypass Security Hardening (17 Audited Vulnerabilities)**:
  - **SEC-01 (JWT Path Traversal)**: Implemented recursive percent-decoding (`decode_path`) and RFC path normalization (`/./`, `/../`, `%2e%2e`, `%252e%252e`) in `nexuslb-wasm`.
  - **SEC-02 (Authenticated Cache Isolation)**: Enforced RFC 7234 §3.2 so requests with `Authorization` headers bypass cache, and responses to authorized requests are never stored without explicit `public` directives.
  - **SEC-03 (Sliding Window DoS Defense)**: Hardened rate limiter against key-rotation DDoS with `MAX_SW_KEYS = 65,536` cap, periodic stale key sweeps, and oldest-entry eviction.
  - **SEC-04 (AccessLogger Thread Lifecycle)**: Fixed background thread leak on dynamic reload with graceful disconnect handling and `Drop` atomic shutdown.
  - **SEC-05 (Keep-Alive Host Switching)**: Enforced RFC 9112 §3.2 rejecting attempts to switch `Host` on active keep-alive streams with `421 Misdirected Request`.
  - **SEC-06 (HTTPS 301 Sanitization)**: Sanitized redirect headers to strip CR/LF/control characters and prevent protocol-relative (`//`) open redirects.
  - **SEC-07 (Domain Suffix Route Hijacking)**: Enforced boundary dot checks in `HostMatch::Suffix` preventing `evilexample.com` from matching `.example.com`.
  - **SEC-08 (Side-Channel Hardening)**: Upgraded `diff` accumulator in `constant_time_eq` to `usize` to prevent modulo-256 truncation; restricted unauthenticated health probe bypass strictly to `GET` and `HEAD`.
  - **SEC-09 & SEC-10 (HTTP/2 Stream Protection)**: Added 5s connect timeouts and 30s per-chunk streaming timeouts in `nexuslb-proxy/h2.rs`, with `DriverGuard` RAII task cancellation.
  - **SEC-11 (TCP Session Lifetime)**: Enforced `MAX_TCP_SESSION_DURATION` (1-hour cap) to prevent indefinite connection slot consumption.
  - **SEC-12 (ConnectionPool Reference Cycles)**: Converted background idle sweep references from strong `Arc` to `Weak` to ensure clean pool teardown.
  - **SEC-13 (HTTP Request Smuggling)**: Rejected conflicting `Content-Length` and `Transfer-Encoding` headers or chunked transfer on non-supporting endpoints with `400 Bad Request`.
  - **SEC-14 (JWT Payload & CRLF Defense)**: Enforced strict JSON schema validation for JWT payloads and sanitized downstream identity headers against header injection.
  - **SEC-15 (Circuit Breaker Cooldown Isolation)**: Added per-backend atomic cooldown tracking so one tripping backend cannot delay recovery for others.
  - **SEC-16 (Health Checker Injection Defense)**: Stripped CR/LF characters from probe paths and enforced exact expected HTTP status codes.
  - **SEC-17 (WRR State Memory Bound)**: Added retention sweep to clean up de-registered backends from the weighted round-robin scheduling table under churn.
- **Empirical Real-Time Telemetry & Showdown**:
  - Live benchmark verified at **114,373 req/sec** sustained throughput with **0 failed requests** across 676,000+ requests.
  - Sub-millisecond latency profile: **$p50 = 500\text{ µs}$** and **$p99 = 1.0\text{ ms}$**.
  - Flatline resident memory (RSS: 11.0 MB) throughout continuous high-concurrency keep-alive load.
  - Updated web portal and showdown tables with live empirical telemetry.

## [0.0.3] - 2026-09-26

### Added & Hardened
- **Enterprise Security Hardening**:
  - Constant-time token verification (`subtle::ConstantTimeEq`) across all Admin REST API endpoints to mitigate timing side-channel attacks.
  - Granular RBAC permissions distinguishing read-only operations from mutation operations (`write` scope required for backend draining and configuration reloads).
  - Standardized secret redaction (`to_redacted()`) ensuring TLS private keys, admin bearer tokens, and credentials in `/config` API dumps and log output are replaced with `"[REDACTED]"`.
  - Enforced secure default posture in sample configuration: `admin.enabled: false` and `admin.token: ""` requiring deliberate operator activation.
- **Resilient Dataplane & Streaming**:
  - Bound header parsing limits (maximum 64 KB header buffer, up to 96 headers parsed via `httparse`) with HTTP `431 Request Header Fields Too Large` rejection on overflow.
  - Proper HTTP syntax validation returning RFC-compliant `400 Bad Request` on malformed requests.
  - Decoupled request body streaming with maximum 16 MB body threshold to eliminate out-of-memory attack surfaces.
- **Sampled P2C Adaptive Scheduler**:
  - Replaced $O(N)$ full backend scanning with bounded $O(k)$ Power-of-Two-Choices sampling ($k = \min(2, N)$), reducing lock hold time and CPU cache thrashing in ultra-large backend clusters.
  - Completely purged UltraBalancer marketing claims in favor of empirical comparisons against commercial solutions (NGINX Plus $3,500/yr, HAProxy Enterprise, Envoy Enterprise).
- **Chaos & Failure Modes Verification Suite**:
  - Comprehensive 7-test stability suite (`failure_modes_and_stability_test.rs`) covering:
    1. Slowloris / partial byte hangup resilience
    2. Broken / refusing upstream handling and graceful recovery
    3. Fuzzed and corrupt HTTP input rejection
    4. Circuit breaker cascade isolation (trips to `Open`, router pool excludes)
    5. Oversized headers (>64 KB) rejected with `431`
    6. Admin API security, RBAC mutation denial (403), constant-time token verification, and config redaction
    7. Pipelined keep-alive stress with mixed bodies
- **Benchmark Consistency**:
  - Canonicalized throughput and latency figures across website, showdown tables, and documentation: C=50 (112,518 req/s), C=100 (118,872 req/s, 525 µs P50, 1.37 ms P99, 2.6 MB RSS), C=250 (127,009 req/s).

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
