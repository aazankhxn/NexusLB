use ahash::AHashMap;
use parking_lot::RwLock;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

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
