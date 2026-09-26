use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

/// High-performance lock-free atomic statistics tracking for a backend.
/// Designed for zero-contention atomic updates in the request hot path.
#[derive(Debug)]
pub struct AtomicBackendStats {
    active_connections: AtomicU64,
    total_connections: AtomicU64,
    total_requests: AtomicU64,
    total_responses: AtomicU64,
    total_errors: AtomicU64,
    consecutive_errors: AtomicU32,
    consecutive_successes: AtomicU32,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    latency_sum_micros: AtomicU64,
    latency_count: AtomicU64,
    ewma_latency_micros: AtomicU64,
    min_latency_micros: AtomicU64,
    max_latency_micros: AtomicU64,
    cached_score: AtomicU64,
    _last_update_millis: AtomicU64,
}

impl Default for AtomicBackendStats {
    fn default() -> Self {
        Self::new()
    }
}

impl AtomicBackendStats {
    pub const fn new() -> Self {
        Self {
            active_connections: AtomicU64::new(0),
            total_connections: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
            total_responses: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            consecutive_errors: AtomicU32::new(0),
            consecutive_successes: AtomicU32::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            latency_sum_micros: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
            ewma_latency_micros: AtomicU64::new(0),
            min_latency_micros: AtomicU64::new(u64::MAX),
            max_latency_micros: AtomicU64::new(0),
            cached_score: AtomicU64::new(100),
            _last_update_millis: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    pub fn inc_active_connections(&self) -> u64 {
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed) + 1
    }

    #[inline(always)]
    pub fn dec_active_connections(&self) -> u64 {
        let prev = self.active_connections.fetch_sub(1, Ordering::Relaxed);
        if prev == 0 {
            // Guard against underflow
            self.active_connections.store(0, Ordering::Relaxed);
            0
        } else {
            prev - 1
        }
    }

    #[inline(always)]
    pub fn active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn inc_requests(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn total_errors(&self) -> u64 {
        self.total_errors.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn total_responses(&self) -> u64 {
        self.total_responses.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn record_success(&self, latency: Duration, bytes_in: u64, bytes_out: u64) {
        self.total_responses.fetch_add(1, Ordering::Relaxed);
        self.consecutive_errors.store(0, Ordering::Relaxed);
        self.consecutive_successes.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes_in, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes_out, Ordering::Relaxed);

        let micros = latency.as_micros() as u64;
        self.latency_sum_micros.fetch_add(micros, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);

        // Update min/max latency lock-free
        let _ = self.min_latency_micros.fetch_min(micros, Ordering::Relaxed);
        let _ = self.max_latency_micros.fetch_max(micros, Ordering::Relaxed);

        // Update EWMA: alpha = 0.2 (represented as integer arithmetic: ewma = (4 * ewma + micros) / 5)
        let mut current_ewma = self.ewma_latency_micros.load(Ordering::Relaxed);
        loop {
            let next_ewma = if current_ewma == 0 {
                micros
            } else {
                (current_ewma * 4 + micros) / 5
            };
            match self.ewma_latency_micros.compare_exchange_weak(
                current_ewma,
                next_ewma,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_ewma = actual,
            }
        }
    }

    #[inline(always)]
    pub fn record_error(&self) {
        self.total_errors.fetch_add(1, Ordering::Relaxed);
        self.consecutive_successes.store(0, Ordering::Relaxed);
        self.consecutive_errors.fetch_add(1, Ordering::Relaxed);
        // Penalty for error in EWMA: artificially increase latency estimate
        let mut current_ewma = self.ewma_latency_micros.load(Ordering::Relaxed);
        loop {
            let next_ewma = if current_ewma == 0 {
                100_000 // 100ms penalty
            } else {
                current_ewma.saturating_add(50_000)
            };
            match self.ewma_latency_micros.compare_exchange_weak(
                current_ewma,
                next_ewma,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_ewma = actual,
            }
        }
    }

    #[inline(always)]
    pub fn ewma_latency(&self) -> Duration {
        Duration::from_micros(self.ewma_latency_micros.load(Ordering::Relaxed))
    }

    #[inline(always)]
    pub fn average_latency(&self) -> Duration {
        let count = self.latency_count.load(Ordering::Relaxed);
        let sum = self.latency_sum_micros.load(Ordering::Relaxed);
        let micros = sum.checked_div(count).unwrap_or(0);
        Duration::from_micros(micros)
    }

    #[inline(always)]
    pub fn consecutive_errors(&self) -> u32 {
        self.consecutive_errors.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn consecutive_successes(&self) -> u32 {
        self.consecutive_successes.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn cached_score(&self) -> u64 {
        self.cached_score.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn set_cached_score(&self, score: u64) {
        self.cached_score.store(score, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> BackendStatsSnapshot {
        let count = self.latency_count.load(Ordering::Relaxed);
        let sum = self.latency_sum_micros.load(Ordering::Relaxed);
        let min = self.min_latency_micros.load(Ordering::Relaxed);
        let max = self.max_latency_micros.load(Ordering::Relaxed);

        let avg_micros = sum.checked_div(count).unwrap_or(0);
        let min_micros = if min == u64::MAX { 0 } else { min };

        BackendStatsSnapshot {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_connections: self.total_connections.load(Ordering::Relaxed),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_responses: self.total_responses.load(Ordering::Relaxed),
            total_errors: self.total_errors.load(Ordering::Relaxed),
            consecutive_errors: self.consecutive_errors.load(Ordering::Relaxed),
            consecutive_successes: self.consecutive_successes.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            avg_latency_micros: avg_micros,
            ewma_latency_micros: self.ewma_latency_micros.load(Ordering::Relaxed),
            min_latency_micros: min_micros,
            max_latency_micros: max,
            score: self.cached_score.load(Ordering::Relaxed),
        }
    }
}

/// Point-in-time snapshot of backend statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendStatsSnapshot {
    pub active_connections: u64,
    pub total_connections: u64,
    pub total_requests: u64,
    pub total_responses: u64,
    pub total_errors: u64,
    pub consecutive_errors: u32,
    pub consecutive_successes: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub avg_latency_micros: u64,
    pub ewma_latency_micros: u64,
    pub min_latency_micros: u64,
    pub max_latency_micros: u64,
    pub score: u64,
}

impl BackendStatsSnapshot {
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_errors as f64 / self.total_requests as f64
        }
    }
}
