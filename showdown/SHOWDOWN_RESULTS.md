# Head-to-Head Benchmark: NexusLB vs. UltraBalancer

Identical local testing conditions (Apple Silicon M-Series, 8 Cores, identical loopback mock backends).

## 1. Quantitative Performance Matrix

| Metric | UltraBalancer v3.0.0 | NexusLB v0.1.0 | Advantage |
| :--- | :--- | :--- | :--- |
| **Throughput (C=50)** | 89137.9 req/s | **123828.0 req/s** | NexusLB |
| **Throughput (C=100)** | 95262.0 req/s | **118814.4 req/s** | NexusLB |
| **Throughput (C=250)** | 103988.2 req/s | **106934.8 req/s** | NexusLB |
| **Median Latency (P50)** | 1012 µs | **810 µs** | NexusLB |
| **P90 Latency** | 1414 µs | **877 µs** | NexusLB |
| **Tail Latency (P99)** | 1935 µs | **1288 µs** | NexusLB |
| **Resident Memory (RSS)** | 35.8 MB | **37.0 MB** | UltraBalancer |

## 2. Qualitative Architectural & Feature Comparison

| Feature / Capability | UltraBalancer | NexusLB | Advantage |
| :--- | :--- | :--- | :--- |
| **Pluggable I/O Engines** | Tokio only | **Tokio, io_uring, AF_XDP** | **NexusLB** (Multi-engine architecture) |
| **Scheduling Algorithms** | 7 algorithms | **10 algorithms** (includes PeakEWMA, Adaptive scoring) | **NexusLB** |
| **Operator Experience** | Web browser only | **Interactive TUI Dashboard (`nexuslb top`)** | **NexusLB** |
| **Distributed Tracing** | None | **W3C `traceparent` OpenTelemetry** | **NexusLB** |
| **RFC 7234 In-Memory Cache** | Key-value cache | **Conditional ETag revalidation (304 Not Modified)** | **NexusLB** |
| **Service Discovery** | Static config | **K8s Endpoints, DNS, File Catalog Discovery** | **NexusLB** |
| **Zero-Trust Security** | Basic TLS | **Mutual TLS (mTLS) with SHA-256 fingerprinting** | **NexusLB** |
| **Filter Extensibility** | Built-in middlewares | **Pipeline Filters (JWT Auth, Header Rewrites)** | **NexusLB** |
| **Zero-Copy Splice** | No | **Linux Kernel `splice` pipeline** | **NexusLB** |
