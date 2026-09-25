# NexusLB Technical Architecture

This document provides a deep-dive analysis into the design, synchronization primitives, memory layout, and execution pipelines of **NexusLB**.

---

## 1. Core Architectural Tenets

NexusLB's primary design goal is **maximizing throughput and minimizing latency variance** under high connection concurrency.

Key architectural decisions:
1. **Thread-per-core Worker Model**: Workers run on dedicated OS threads bound to logical CPU cores (`nexuslb-runtime`).
2. **Kernel Dispatch via `SO_REUSEPORT`**: Sockets are bound per-worker, allowing the Linux/Darwin kernel to distribute incoming connections across workers without userspace synchronization.
3. **Wait-Free Read Dataplane**: The request hot path reads immutable configuration and routing state via `ArcSwap` pointer swaps.
4. **Cacheline Isolation**: Thread-local counters and worker statistics are aligned to 64-byte boundaries (`#[repr(align(64))]`) to eliminate false sharing.
5. **Zero-Allocation I/O Buffering**: Packets and HTTP chunks are streamed using a lock-free `BufferPool` backed by crossbeam array queues.

---

## 2. Request Hot Path Lifecycle

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

### Hot-Path Guarantees:
- **No Global Mutexes**: Schedulers, routing tables, and backends contain zero global mutexes or rwlocks in the request path.
- **No Per-Request Heap Allocations**: Data streaming reuses pre-allocated 32KB buffers from the pool.
- **No Synchronous Filesystem Access**: Logging is non-blocking and sampled. Default production logs only emit at `WARN` or `ERROR` levels.

---

## 3. Lock-Free Statistics & Adaptive Scoring

Each backend maintains an `AtomicBackendStats` structure:
- Active connections: `AtomicU64`
- Total requests / total responses / total errors: `AtomicU64`
- Consecutive errors / successes: `AtomicU32`
- Exponential Weighted Moving Average (EWMA) Latency: `AtomicU64`

### EWMA Decay Formula:
Instead of floating-point operations in the hot path, EWMA is calculated via fixed-point integer arithmetic:

$$\text{EWMA}_{t} = \frac{4 \times \text{EWMA}_{t-1} + \text{Latency}_{\text{micros}}}{5}$$

Updated via a lock-free `compare_exchange_weak` CAS loop.

### Adaptive Cost Scoring:
$$\text{Score} = \frac{\text{LatencyCost} + \text{LoadCost} + \text{ErrorCost} + \text{HealthCost}}{\text{Weight}}$$

When a backend suffers elevated error rates or tail latency spikes, its score increases immediately, causing the scheduler to reroute traffic to healthy peers without waiting for a manual operator intervention or slow active health check probe.

---

## 4. Zero-Downtime State Swapping

Zero-downtime configuration reload works via atomic pointer swapping:

1. New configuration file is loaded and parsed.
2. Validation ensures all addresses, pools, algorithms, and durations are valid.
3. A new `DataplaneState` instance is constructed.
4. `SharedDataplaneState::swap()` updates the atomic pointer (`ArcSwap::store`).
5. New connections immediately use the updated state.
6. Existing in-flight connections continue on the previous `Arc<DataplaneState>` until complete.
7. Removed backends are drained gracefully by `DrainController`.

---

## 5. Security & Isolation

- **Admin API**: Isolated on separate port (`127.0.0.1:9091`), protected by Bearer token authentication.
- **Memory Safety**: 100% safe Rust. No raw pointer dereferences or unsafe memory blocks in the proxying pipeline.
- **Resource Sandboxing**: Systemd unit restricts kernel tunables, drops non-essential capabilities, and isolates filesystem access.
