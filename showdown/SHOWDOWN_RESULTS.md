# Head-to-Head Benchmark: NexusLB vs. UltraBalancer

Identical local testing conditions (Apple Silicon M-Series, 8 Cores, identical loopback mock backends).

## 1. Quantitative Performance Matrix

| Metric | UltraBalancer v3.0.0 | NexusLB v0.1.0 | Advantage |
| :--- | :--- | :--- | :--- |
| **Throughput (C=50)** | 94739.4 req/s | **134281.9 req/s** | NexusLB |
| **Throughput (C=100)** | 96441.2 req/s | **132472.9 req/s** | NexusLB |
| **Throughput (C=250)** | 100000.1 req/s | **127009.3 req/s** | NexusLB |
| **Median Latency (P50)** | 1007 µs | **753 µs** | NexusLB |
| **P90 Latency** | 1411 µs | **794 µs** | NexusLB |
| **Tail Latency (P99)** | 1934 µs | **846 µs** | NexusLB |
| **Resident Memory (RSS)** | 39.5 MB | **16.5 MB** | NexusLB |

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
