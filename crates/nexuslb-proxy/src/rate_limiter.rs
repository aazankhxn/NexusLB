use ahash::AHashMap;
use parking_lot::RwLock;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum RateLimitError {
    #[error("Rate limit sync failed: {0}")]
    SyncFailed(String),
    #[error("Cluster coordinator unreachable")]
    CoordinatorUnreachable,
}

// ============================================================================
// 1. TOKEN BUCKET (Lock-Free)
// ============================================================================

/// High-performance lock-free TokenBucket rate limiter
pub struct TokenBucket {
    capacity: u64,
    refill_rate_per_sec: u64,
    tokens: AtomicU64,
    last_refill: AtomicU64,
    start_instant: Instant,
}

impl TokenBucket {
    pub fn new(rate_per_sec: u64, burst_capacity: u64) -> Self {
        Self {
            capacity: burst_capacity,
            refill_rate_per_sec: rate_per_sec,
            tokens: AtomicU64::new(burst_capacity),
            last_refill: AtomicU64::new(0),
            start_instant: Instant::now(),
        }
    }

    #[inline(always)]
    pub fn try_acquire(&self, count: u64) -> bool {
        let now_nanos = self.start_instant.elapsed().as_nanos() as u64;
        let last = self.last_refill.load(Ordering::Relaxed);

        // Refill tokens based on elapsed time
        let elapsed_nanos = now_nanos.saturating_sub(last);
        if elapsed_nanos >= 1_000_000 {
            // At least 1ms elapsed
            let added_tokens = (elapsed_nanos * self.refill_rate_per_sec) / 1_000_000_000;
            if added_tokens > 0
                && self
                    .last_refill
                    .compare_exchange_weak(last, now_nanos, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
            {
                let mut current = self.tokens.load(Ordering::Relaxed);
                loop {
                    let new_val = (current + added_tokens).min(self.capacity);
                    match self.tokens.compare_exchange_weak(
                        current,
                        new_val,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(actual) => current = actual,
                    }
                }
            }
        }

        // Try to deduct `count` tokens
        let mut current = self.tokens.load(Ordering::Relaxed);
        loop {
            if current < count {
                return false;
            }
            match self.tokens.compare_exchange_weak(
                current,
                current - count,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }
}

// ============================================================================
// 2. SLIDING WINDOW COUNTER
// ============================================================================

/// High-precision Sliding Window Counter rate limiter.
/// Solves the edge-of-window bursting problem by weighting counts between
/// the previous and current window slices.
pub struct SlidingWindowCounter {
    window_duration: Duration,
    max_requests: u64,
    state: RwLock<SlidingWindowState>,
}

struct SlidingWindowState {
    current_window_start: Instant,
    current_count: u64,
    previous_count: u64,
}

impl SlidingWindowCounter {
    pub fn new(window_duration: Duration, max_requests: u64) -> Self {
        Self {
            window_duration,
            max_requests,
            state: RwLock::new(SlidingWindowState {
                current_window_start: Instant::now(),
                current_count: 0,
                previous_count: 0,
            }),
        }
    }

    pub fn try_acquire(&self, count: u64) -> bool {
        let now = Instant::now();
        let mut state = self.state.write();

        let elapsed = now.saturating_duration_since(state.current_window_start);
        let window_nanos = self.window_duration.as_nanos().max(1) as f64;

        if elapsed >= self.window_duration {
            let windows_passed = elapsed.as_nanos() / self.window_duration.as_nanos().max(1);
            if windows_passed == 1 {
                state.previous_count = state.current_count;
                state.current_count = 0;
                state.current_window_start += self.window_duration;
            } else {
                state.previous_count = 0;
                state.current_count = 0;
                state.current_window_start = now;
            }
        }

        let time_into_window = now
            .saturating_duration_since(state.current_window_start)
            .as_nanos() as f64;
        let prev_weight = ((window_nanos - time_into_window) / window_nanos).clamp(0.0, 1.0);

        let estimated_count =
            (state.previous_count as f64 * prev_weight) + state.current_count as f64;

        if estimated_count + (count as f64) <= self.max_requests as f64 {
            state.current_count += count;
            true
        } else {
            false
        }
    }

    pub fn estimated_usage(&self) -> u64 {
        let now = Instant::now();
        let state = self.state.read();

        let elapsed = now.saturating_duration_since(state.current_window_start);
        if elapsed >= self.window_duration * 2 {
            return 0;
        }

        let window_nanos = self.window_duration.as_nanos().max(1) as f64;
        let time_into_window = elapsed.as_nanos() as f64;
        let prev_weight = ((window_nanos - time_into_window) / window_nanos).clamp(0.0, 1.0);

        ((state.previous_count as f64 * prev_weight) + state.current_count as f64).round() as u64
    }
}

/// Multi-key sliding-window rate limiter (e.g., per-client IP or API token).
pub struct SlidingWindowRateLimiter {
    window_duration: Duration,
    max_per_window: u64,
    counters: RwLock<AHashMap<String, Arc<SlidingWindowCounter>>>,
}

impl SlidingWindowRateLimiter {
    pub fn new(window_duration: Duration, max_per_window: u64) -> Self {
        Self {
            window_duration,
            max_per_window,
            counters: RwLock::new(AHashMap::new()),
        }
    }

    pub fn check_key(&self, key: &str, count: u64) -> bool {
        let counter = {
            let map = self.counters.read();
            map.get(key).cloned()
        };

        let counter = match counter {
            Some(c) => c,
            None => {
                let mut map = self.counters.write();
                map.entry(key.to_string())
                    .or_insert_with(|| {
                        Arc::new(SlidingWindowCounter::new(
                            self.window_duration,
                            self.max_per_window,
                        ))
                    })
                    .clone()
            }
        };

        counter.try_acquire(count)
    }

    pub fn check_ip(&self, ip: IpAddr) -> bool {
        self.check_key(&ip.to_string(), 1)
    }
}

// ============================================================================
// 3. DISTRIBUTED CLUSTER RATE LIMITER
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterQuotaStatus {
    pub allowed: bool,
    pub current_usage: u64,
    pub remaining_quota: u64,
}

/// Trait implemented by cluster synchronization coordinators (e.g. Redis, etcd, Raft peers).
#[async_trait::async_trait]
pub trait DistributedQuotaSync: Send + Sync {
    async fn sync_usage(&self, key: &str, delta: u64)
        -> Result<ClusterQuotaStatus, RateLimitError>;
    async fn get_usage(&self, key: &str) -> Result<u64, RateLimitError>;
    async fn reset_key(&self, key: &str) -> Result<(), RateLimitError>;
}

/// Thread-safe in-memory cluster coordinator for tests and multi-worker NexusLB topologies.
pub struct InMemoryClusterCoordinator {
    max_quota: u64,
    store: RwLock<AHashMap<String, u64>>,
}

impl InMemoryClusterCoordinator {
    pub fn new(max_quota: u64) -> Self {
        Self {
            max_quota,
            store: RwLock::new(AHashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl DistributedQuotaSync for InMemoryClusterCoordinator {
    async fn sync_usage(
        &self,
        key: &str,
        delta: u64,
    ) -> Result<ClusterQuotaStatus, RateLimitError> {
        let mut map = self.store.write();
        let entry = map.entry(key.to_string()).or_insert(0);
        let new_val = *entry + delta;

        if new_val <= self.max_quota {
            *entry = new_val;
            Ok(ClusterQuotaStatus {
                allowed: true,
                current_usage: new_val,
                remaining_quota: self.max_quota.saturating_sub(new_val),
            })
        } else {
            Ok(ClusterQuotaStatus {
                allowed: false,
                current_usage: *entry,
                remaining_quota: self.max_quota.saturating_sub(*entry),
            })
        }
    }

    async fn get_usage(&self, key: &str) -> Result<u64, RateLimitError> {
        let map = self.store.read();
        Ok(map.get(key).copied().unwrap_or(0))
    }

    async fn reset_key(&self, key: &str) -> Result<(), RateLimitError> {
        let mut map = self.store.write();
        map.remove(key);
        Ok(())
    }
}

/// Cluster-aware rate limiter combining sub-microsecond local decisions with
/// asynchronous distributed cluster synchronization.
pub struct ClusterRateLimiter {
    local_limiter: SlidingWindowCounter,
    cluster_coordinator: Arc<dyn DistributedQuotaSync>,
    local_pending_delta: AtomicU64,
}

impl ClusterRateLimiter {
    pub fn new(
        window_duration: Duration,
        local_max: u64,
        cluster_coordinator: Arc<dyn DistributedQuotaSync>,
    ) -> Self {
        Self {
            local_limiter: SlidingWindowCounter::new(window_duration, local_max),
            cluster_coordinator,
            local_pending_delta: AtomicU64::new(0),
        }
    }

    /// Fast-path local admission check.
    pub fn try_acquire_local(&self, count: u64) -> bool {
        if self.local_limiter.try_acquire(count) {
            self.local_pending_delta.fetch_add(count, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Flush accumulated local deltas to the distributed cluster coordinator.
    pub async fn flush_to_cluster(&self, key: &str) -> Result<ClusterQuotaStatus, RateLimitError> {
        let delta = self.local_pending_delta.swap(0, Ordering::Relaxed);
        if delta == 0 {
            let current = self.cluster_coordinator.get_usage(key).await?;
            return Ok(ClusterQuotaStatus {
                allowed: true,
                current_usage: current,
                remaining_quota: 0,
            });
        }

        self.cluster_coordinator.sync_usage(key, delta).await
    }
}

// ============================================================================
// 4. BASELINE RATE LIMITER (Existing compatibility)
// ============================================================================

pub struct RateLimiter {
    global_bucket: Option<TokenBucket>,
    ip_buckets: RwLock<AHashMap<IpAddr, Arc<TokenBucket>>>,
    client_rps: Option<u64>,
}

impl RateLimiter {
    pub fn new(global_rps: Option<u32>, client_rps: Option<u32>) -> Self {
        let global_bucket = global_rps.map(|rps| TokenBucket::new(rps as u64, (rps * 2) as u64));
        Self {
            global_bucket,
            ip_buckets: RwLock::new(AHashMap::new()),
            client_rps: client_rps.map(|r| r as u64),
        }
    }

    #[inline(always)]
    pub fn check(&self, client_ip: Option<IpAddr>) -> bool {
        // Check global rate limit
        if let Some(ref global) = self.global_bucket {
            if !global.try_acquire(1) {
                return false;
            }
        }

        // Check per-client IP rate limit
        if let (Some(rps), Some(ip)) = (self.client_rps, client_ip) {
            let bucket = {
                let map = self.ip_buckets.read();
                map.get(&ip).cloned()
            };

            let bucket = match bucket {
                Some(b) => b,
                None => {
                    let mut map = self.ip_buckets.write();
                    map.entry(ip)
                        .or_insert_with(|| Arc::new(TokenBucket::new(rps, rps * 2)))
                        .clone()
                }
            };

            if !bucket.try_acquire(1) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_acquire() {
        let bucket = TokenBucket::new(10, 5);
        for _ in 0..5 {
            assert!(bucket.try_acquire(1));
        }
        // Burst capacity exhausted
        assert!(!bucket.try_acquire(1));
    }

    #[test]
    fn test_sliding_window_counter() {
        let limiter = SlidingWindowCounter::new(Duration::from_millis(500), 5);
        for _ in 0..5 {
            assert!(limiter.try_acquire(1));
        }
        assert!(!limiter.try_acquire(1));
    }

    #[test]
    fn test_sliding_window_rate_limiter_keys() {
        let limiter = SlidingWindowRateLimiter::new(Duration::from_secs(1), 3);
        assert!(limiter.check_key("user_alice", 1));
        assert!(limiter.check_key("user_alice", 1));
        assert!(limiter.check_key("user_alice", 1));
        assert!(!limiter.check_key("user_alice", 1));

        // Bob has his own independent quota
        assert!(limiter.check_key("user_bob", 1));
        assert!(limiter.check_key("user_bob", 1));
    }

    #[tokio::test]
    async fn test_distributed_cluster_rate_limiter() {
        let coordinator = Arc::new(InMemoryClusterCoordinator::new(10));
        let node_a = ClusterRateLimiter::new(Duration::from_secs(1), 5, coordinator.clone());
        let node_b = ClusterRateLimiter::new(Duration::from_secs(1), 5, coordinator.clone());

        assert!(node_a.try_acquire_local(4));
        assert!(node_b.try_acquire_local(4));

        let res_a = node_a.flush_to_cluster("global_quota").await.unwrap();
        assert!(res_a.allowed);
        assert_eq!(res_a.current_usage, 4);

        let res_b = node_b.flush_to_cluster("global_quota").await.unwrap();
        assert!(res_b.allowed);
        assert_eq!(res_b.current_usage, 8);

        // Third node pushes 3 requests over 10 quota limit
        let res_c = coordinator.sync_usage("global_quota", 3).await.unwrap();
        assert!(!res_c.allowed);
        assert_eq!(res_c.current_usage, 8);
    }
}
