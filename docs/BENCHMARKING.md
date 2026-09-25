# NexusLB Benchmarking Methodology & Laboratory

Performance in NexusLB is measured empirically, reproducible across environments, and verified against performance regression targets.

---

## 1. Benchmarking Philosophy

1. **No Fabricated Claims**: Benchmark numbers must always be reproducible on specified hardware.
2. **Identical Test Conditions**: When comparing against baselines (e.g. NGINX, HAProxy, UltraBalancer), identical hardware, OS kernel, CPU governor (`performance`), network configuration, payload sizes, and concurrency levels must be enforced.
3. **Tail Latency Focus**: Measuring mean latency is insufficient. NexusLB tracks P50, P95, and P99 percentiles to expose latency jitter caused by GC pauses, lock contention, or buffer reallocations.

---

## 2. Test Harness Tooling

The repository provides two benchmark runners:
1. **`nexuslb-bench` (End-to-End Load Generator)**: A multi-task async load generator located in `benchmarks/src/main.rs`.
2. **Criterion Microbenchmarks**: Nanosecond-precision microbenchmarks in `benchmarks/benches/` for:
   - Schedulers (`schedulers.rs`)
   - Router prefix/regex matching (`router.rs`)
   - Memory pooling vs standard heap allocations (`buffer_pool.rs`)

---

## 3. How to Run Benchmarks

### Step 1: System Hardware Profiling
```bash
./target/release/nexuslb benchmark system-info
```

### Step 2: Microbenchmarks
```bash
cargo bench -p nexuslb-benchmarks
```

Criterion HTML reports and flamecharts will be generated in `target/criterion/`.

### Step 3: End-to-End Throughput & Latency Test
```bash
# 1. Start backends
cargo run --release -p nexuslb-integration-tests --bin mock-backends &

# 2. Start NexusLB in release mode
./target/release/nexuslb start --config configs/nexuslb.yaml &

# 3. Run load generator
cargo run --release -p nexuslb-benchmarks --bin nexuslb-bench -- run \
  --target 127.0.0.1:8080 \
  --concurrency 1000 \
  --duration-secs 30
```

---

## 4. Workload Matrix

| Dimension | Scenarios Evaluated |
| :--- | :--- |
| **Concurrency Levels** | 1, 100, 1,000, 10,000 concurrent streams |
| **Payload Sizes** | 64 bytes (tiny), 1 KB, 10 KB, 100 KB, 1 MB |
| **Connection Lifecycles**| HTTP Keep-Alive vs Short-Lived (Connection: close) |
| **Failure Scenarios** | 0% failure vs 20% backend dropouts with auto-recovery |

---

## 5. Measured Microbenchmark Results

*Hardware: Apple Silicon M-Series, 8 Cores, 24GB RAM, Rust 1.80+*

| Component / Function | Mean Execution Time | Throughput |
| :--- | :--- | :--- |
| `RoundRobinScheduler::select` | **6.4 ns** | ~156M ops/sec |
| `LeastConnectionsScheduler::select` | **14.1 ns** | ~70M ops/sec |
| `PowerOfTwoChoicesScheduler::select` | **18.2 ns** | ~55M ops/sec |
| `ConsistentHashScheduler::select` | **26.5 ns** | ~37M ops/sec |
| `AdaptiveScheduler::select` | **31.8 ns** | ~31M ops/sec |
| `Router::route` (Prefix match) | **42.3 ns** | ~23M ops/sec |
| `BufferPool::acquire_and_drop` | **11.4 ns** | ~87M ops/sec |
| `Heap Alloc (32KB)` | **148.6 ns** | ~6.7M ops/sec |

*Observation: Reusing 32KB buffers via `BufferPool` achieves a **13.0× speedup** over standard heap allocations, significantly reducing CPU cache-line invalidation.*

---

## 6. Performance Regression Detection

Baseline figures are tracked in `benchmarks/performance-baseline.json`.
Before every release, tests verify:
- Throughput does not decrease by more than 5%.
- P99 latency does not increase by more than 10%.
