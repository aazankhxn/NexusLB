# NexusLB Scheduling Algorithms

NexusLB implements **10 production scheduling algorithms** optimized for different workload characteristics, from microservice RPCs to heavy streaming media and stateful sessions.

---

## 1. Overview of Algorithms

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

---

## 2. In-Depth Details

### 1. `adaptive` (Default)
Combines **EWMA moving average latency**, **active connection load**, and **historical error rates** into a unified composite penalty score:
$$\text{Score} = \text{EWMA Latency} \times (1 + \text{Active Connections}) \times \left(1 + \frac{\text{Recent Errors}}{10}\right)$$
The backend with the lowest score is selected. If a node starts slowing down due to garbage collection or background batch jobs, NexusLB automatically shifts traffic away before the node fails health checks.

### 2. `power_of_two_choices` (P2C)
Eliminates the "thundering herd" problem seen in Least Connections across multi-threaded workers:
1. Picks two available backends at random.
2. Compares active connection counts and recent latency.
3. Dispatches to the superior candidate.
Mathematically proven (Mitzenmacher et al.) to achieve near-optimal load distribution with zero lock contention ($O(1)$).

### 3. `ewma_latency` (Peak EWMA)
Maintains an Exponentially Weighted Moving Average of request latencies:
$$\text{EWMA}_t = \alpha \cdot \text{Latency}_t + (1 - \alpha) \cdot \text{EWMA}_{t-1}$$
Gives higher weight to recent latency spikes while dampening transient jitter.

### 4. `consistent_hash` (Ketama Ring)
Maps both backends (using 160 virtual nodes per replica) and incoming request keys onto a 32-bit hash ring:
- Ensures request keys (such as `Host`, URL path, or custom header) always map to the exact same backend.
- When a backend node is added or removed, only $\frac{1}{N}$ keys are relocated, avoiding catastrophic cache eviction stampedes.

### 5. `ip_hash`
Hashes client IPv4 or IPv6 addresses using high-speed SipHash to guarantee sticky sessions for clients without requiring cookies or server-side session sync.

---

## 3. Configuration Example

```yaml
load_balancer:
  algorithm: "power_of_two_choices" # Or adaptive, consistent_hash, ewma_latency
  default_pool: "primary"
```
