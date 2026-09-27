#![deny(unsafe_code)]

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const LATENCY_BUCKETS_MICROS: [u64; 14] = [
    100,       // 0.1ms
    500,       // 0.5ms
    1_000,     // 1ms
    2_500,     // 2.5ms
    5_000,     // 5ms
    10_000,    // 10ms
    25_000,    // 25ms
    50_000,    // 50ms
    100_000,   // 100ms
    250_000,   // 250ms
    500_000,   // 500ms
    1_000_000, // 1s
    2_500_000, // 2.5s
    5_000_000, // 5s
];

/// Cacheline-padded per-worker metrics to prevent false-sharing on multi-core systems (128-byte cachelines on Apple Silicon)
#[repr(align(128))]
pub struct WorkerMetrics {
    pub worker_id: usize,
    pub requests_total: AtomicU64,
    pub connections_total: AtomicU64,
    pub active_connections: AtomicI64,
    pub backend_requests_total: AtomicU64,
    pub backend_errors_total: AtomicU64,
    pub retries_total: AtomicU64,
    pub circuit_breaker_trips: AtomicU64,
    pub dropped_connections: AtomicU64,
    pub bytes_received_total: AtomicU64,
    pub bytes_sent_total: AtomicU64,
    pub latency_buckets: [AtomicU64; 14],
    pub latency_sum_micros: AtomicU64,
    pub latency_count: AtomicU64,
}

impl WorkerMetrics {
    pub fn new(worker_id: usize) -> Self {
        Self {
            worker_id,
            requests_total: AtomicU64::new(0),
            connections_total: AtomicU64::new(0),
            active_connections: AtomicI64::new(0),
            backend_requests_total: AtomicU64::new(0),
            backend_errors_total: AtomicU64::new(0),
            retries_total: AtomicU64::new(0),
            circuit_breaker_trips: AtomicU64::new(0),
            dropped_connections: AtomicU64::new(0),
            bytes_received_total: AtomicU64::new(0),
            bytes_sent_total: AtomicU64::new(0),
            latency_buckets: Default::default(),
            latency_sum_micros: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    pub fn inc_requests(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn inc_connections(&self) {
        self.connections_total.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn dec_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn inc_backend_requests(&self) {
        self.backend_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn inc_backend_errors(&self) {
        self.backend_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn inc_retries(&self) {
        self.retries_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn inc_circuit_breaker_trips(&self) {
        self.circuit_breaker_trips.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn inc_dropped_connections(&self) {
        self.dropped_connections.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn add_bytes_received(&self, bytes: u64) {
        self.bytes_received_total
            .fetch_add(bytes, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn add_bytes_sent(&self, bytes: u64) {
        self.bytes_sent_total.fetch_add(bytes, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn record_latency(&self, duration: Duration) {
        let micros = duration.as_micros() as u64;

        // Sample 1-in-4 requests for histogram to reduce multi-core cacheline contention.
        // At >100k rps this still provides statistically accurate percentiles.
        // Use the low bits of latency_count as a cheap counter to avoid an extra atomic load.
        let count = self.latency_count.fetch_add(1, Ordering::Relaxed);
        self.latency_sum_micros.fetch_add(micros, Ordering::Relaxed);

        if (count & 0x03) == 0 {
            // Increment ONLY the first matching bucket (1 atomic operation instead of 14)
            for (i, &bucket) in LATENCY_BUCKETS_MICROS.iter().enumerate() {
                if micros <= bucket {
                    self.latency_buckets[i].fetch_add(4, Ordering::Relaxed);
                    return;
                }
            }
        }
    }
}

/// Global registry managing per-worker metrics and Prometheus output generation
pub struct GlobalMetrics {
    workers: Vec<Arc<WorkerMetrics>>,
}

impl GlobalMetrics {
    pub fn new(num_workers: usize) -> Self {
        let mut workers = Vec::with_capacity(num_workers);
        for id in 0..num_workers {
            workers.push(Arc::new(WorkerMetrics::new(id)));
        }
        Self { workers }
    }

    pub fn worker(&self, id: usize) -> Arc<WorkerMetrics> {
        self.workers[id % self.workers.len()].clone()
    }

    pub fn workers(&self) -> &[Arc<WorkerMetrics>] {
        &self.workers
    }

    pub fn aggregate(&self) -> AggregatedMetrics {
        let mut agg = AggregatedMetrics::default();

        for w in &self.workers {
            agg.requests_total += w.requests_total.load(Ordering::Relaxed);
            agg.connections_total += w.connections_total.load(Ordering::Relaxed);
            let active = w.active_connections.load(Ordering::Relaxed);
            if active > 0 {
                agg.active_connections += active as u64;
            }
            agg.backend_requests_total += w.backend_requests_total.load(Ordering::Relaxed);
            agg.backend_errors_total += w.backend_errors_total.load(Ordering::Relaxed);
            agg.retries_total += w.retries_total.load(Ordering::Relaxed);
            agg.circuit_breaker_trips += w.circuit_breaker_trips.load(Ordering::Relaxed);
            agg.dropped_connections += w.dropped_connections.load(Ordering::Relaxed);
            agg.bytes_received_total += w.bytes_received_total.load(Ordering::Relaxed);
            agg.bytes_sent_total += w.bytes_sent_total.load(Ordering::Relaxed);

            for (i, bucket) in w.latency_buckets.iter().enumerate() {
                agg.latency_buckets[i] += bucket.load(Ordering::Relaxed);
            }
            agg.latency_sum_micros += w.latency_sum_micros.load(Ordering::Relaxed);
            agg.latency_count += w.latency_count.load(Ordering::Relaxed);
        }

        agg
    }

    pub fn render_prometheus(&self) -> String {
        let agg = self.aggregate();
        let mut out = String::with_capacity(4096);

        out.push_str("# HELP nexuslb_requests_total Total number of HTTP/TCP requests processed\n");
        out.push_str("# TYPE nexuslb_requests_total counter\n");
        out.push_str(&format!(
            "nexuslb_requests_total {}\n\n",
            agg.requests_total
        ));

        out.push_str(
            "# HELP nexuslb_connections_total Total number of accepted client connections\n",
        );
        out.push_str("# TYPE nexuslb_connections_total counter\n");
        out.push_str(&format!(
            "nexuslb_connections_total {}\n\n",
            agg.connections_total
        ));

        out.push_str("# HELP nexuslb_active_connections Current active client connections\n");
        out.push_str("# TYPE nexuslb_active_connections gauge\n");
        out.push_str(&format!(
            "nexuslb_active_connections {}\n\n",
            agg.active_connections
        ));

        out.push_str(
            "# HELP nexuslb_backend_requests_total Total requests routed to upstream backends\n",
        );
        out.push_str("# TYPE nexuslb_backend_requests_total counter\n");
        out.push_str(&format!(
            "nexuslb_backend_requests_total {}\n\n",
            agg.backend_requests_total
        ));

        out.push_str("# HELP nexuslb_backend_errors_total Total backend errors encountered\n");
        out.push_str("# TYPE nexuslb_backend_errors_total counter\n");
        out.push_str(&format!(
            "nexuslb_backend_errors_total {}\n\n",
            agg.backend_errors_total
        ));

        out.push_str("# HELP nexuslb_retries_total Total retry attempts dispatched\n");
        out.push_str("# TYPE nexuslb_retries_total counter\n");
        out.push_str(&format!("nexuslb_retries_total {}\n\n", agg.retries_total));

        out.push_str("# HELP nexuslb_circuit_breaker_trips Total times circuit breaker tripped\n");
        out.push_str("# TYPE nexuslb_circuit_breaker_trips counter\n");
        out.push_str(&format!(
            "nexuslb_circuit_breaker_trips {}\n\n",
            agg.circuit_breaker_trips
        ));

        out.push_str("# HELP nexuslb_dropped_connections_total Total connections dropped due to limits/shedding\n");
        out.push_str("# TYPE nexuslb_dropped_connections_total counter\n");
        out.push_str(&format!(
            "nexuslb_dropped_connections_total {}\n\n",
            agg.dropped_connections
        ));

        out.push_str("# HELP nexuslb_bytes_received_total Total bytes received from clients\n");
        out.push_str("# TYPE nexuslb_bytes_received_total counter\n");
        out.push_str(&format!(
            "nexuslb_bytes_received_total {}\n\n",
            agg.bytes_received_total
        ));

        out.push_str("# HELP nexuslb_bytes_sent_total Total bytes sent to clients\n");
        out.push_str("# TYPE nexuslb_bytes_sent_total counter\n");
        out.push_str(&format!(
            "nexuslb_bytes_sent_total {}\n\n",
            agg.bytes_sent_total
        ));

        out.push_str("# HELP nexuslb_request_duration_seconds Request duration histogram\n");
        out.push_str("# TYPE nexuslb_request_duration_seconds histogram\n");
        let mut cumulative_count = 0u64;
        for (i, &bucket) in LATENCY_BUCKETS_MICROS.iter().enumerate() {
            cumulative_count += agg.latency_buckets[i];
            let secs = bucket as f64 / 1_000_000.0;
            out.push_str(&format!(
                "nexuslb_request_duration_seconds_bucket{{le=\"{:.4}\"}} {}\n",
                secs, cumulative_count
            ));
        }
        out.push_str(&format!(
            "nexuslb_request_duration_seconds_bucket{{le=\"+Inf\"}} {}\n",
            agg.latency_count
        ));
        out.push_str(&format!(
            "nexuslb_request_duration_seconds_sum {:.6}\n",
            agg.latency_sum_micros as f64 / 1_000_000.0
        ));
        out.push_str(&format!(
            "nexuslb_request_duration_seconds_count {}\n",
            agg.latency_count
        ));

        out.push_str("# HELP nexuslb_worker_requests_total Requests per worker\n");
        out.push_str("# TYPE nexuslb_worker_requests_total counter\n");
        for w in &self.workers {
            out.push_str(&format!(
                "nexuslb_worker_requests_total{{worker=\"{}\"}} {}\n",
                w.worker_id,
                w.requests_total.load(Ordering::Relaxed)
            ));
        }

        out
    }
}

#[derive(Debug, Default, Clone)]
pub struct AggregatedMetrics {
    pub requests_total: u64,
    pub connections_total: u64,
    pub active_connections: u64,
    pub backend_requests_total: u64,
    pub backend_errors_total: u64,
    pub retries_total: u64,
    pub circuit_breaker_trips: u64,
    pub dropped_connections: u64,
    pub bytes_received_total: u64,
    pub bytes_sent_total: u64,
    pub latency_buckets: [u64; 14],
    pub latency_sum_micros: u64,
    pub latency_count: u64,
}

impl AggregatedMetrics {
    pub fn p50_micros(&self) -> u64 {
        self.estimate_percentile(0.50)
    }

    pub fn p95_micros(&self) -> u64 {
        self.estimate_percentile(0.95)
    }

    pub fn p99_micros(&self) -> u64 {
        self.estimate_percentile(0.99)
    }

    fn estimate_percentile(&self, p: f64) -> u64 {
        if self.latency_count == 0 {
            return 0;
        }
        let target = (self.latency_count as f64 * p) as u64;
        let mut accumulated = 0;
        for (i, &bucket_count) in self.latency_buckets.iter().enumerate() {
            accumulated += bucket_count;
            if accumulated >= target {
                return LATENCY_BUCKETS_MICROS[i];
            }
        }
        *LATENCY_BUCKETS_MICROS.last().unwrap_or(&0)
    }
}
