use crate::traits::{Scheduler, SelectionContext};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::{AlgorithmType, BackendState, CircuitState};
use rand::Rng;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AdaptiveConfig {
    pub latency_weight: f64,
    pub load_weight: f64,
    pub error_weight: f64,
    pub health_weight: f64,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            latency_weight: 1.0,
            load_weight: 2.0,
            error_weight: 5.0,
            health_weight: 10.0,
        }
    }
}

pub struct AdaptiveScheduler {
    config: AdaptiveConfig,
}

impl AdaptiveScheduler {
    pub fn new() -> Self {
        Self {
            config: AdaptiveConfig::default(),
        }
    }

    pub fn with_config(config: AdaptiveConfig) -> Self {
        Self { config }
    }

    #[inline(always)]
    pub fn calculate_score(&self, backend: &Backend) -> f64 {
        let stats = backend.stats();
        let ewma_micros = stats.ewma_latency().as_micros() as f64;
        let latency_ms = (ewma_micros / 1000.0).max(1.0);

        let conns = backend.active_connections() as f64;
        let weight = (backend.weight().max(1)) as f64;
        let capacity_ratio = conns / weight;

        let consec_errors = stats.consecutive_errors() as f64;
        let total_reqs = stats.total_requests();
        let total_errs = stats.total_errors();
        let error_rate = if total_reqs > 0 {
            (total_errs as f64 / total_reqs as f64) * 100.0
        } else {
            0.0
        };

        let latency_comp = latency_ms * self.config.latency_weight;
        let load_comp = capacity_ratio * 15.0 * self.config.load_weight;
        let error_comp = (consec_errors * 25.0 + error_rate * 10.0) * self.config.error_weight;

        let health_comp = match backend.state() {
            BackendState::Up => 0.0,
            BackendState::Starting => 200.0,
            _ => 10_000.0,
        } + match backend.circuit_state() {
            CircuitState::Closed => 0.0,
            CircuitState::HalfOpen => 300.0,
            CircuitState::Open => 10_000.0,
        };

        (latency_comp + load_comp + error_comp + health_comp) / weight
    }
}

impl Default for AdaptiveScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for AdaptiveScheduler {
    fn select(&self, backends: &[Arc<Backend>], _ctx: &SelectionContext) -> Option<Arc<Backend>> {
        if backends.is_empty() {
            return None;
        }

        // For small clusters (<= 16 backends), exhaustive scan has negligible overhead
        // and guarantees globally optimal candidate selection.
        if backends.len() <= 16 {
            let mut min_score = f64::MAX;
            let mut best: Option<&Arc<Backend>> = None;

            for b in backends {
                if !b.is_available() {
                    continue;
                }

                let score = self.calculate_score(b);
                if score < min_score - 1e-6 {
                    min_score = score;
                    best = Some(b);
                } else if (score - min_score).abs() <= 1e-6 {
                    if let Some(prev) = best {
                        if b.stats().total_requests() < prev.stats().total_requests() {
                            min_score = score;
                            best = Some(b);
                        }
                    }
                }
            }

            return best.cloned();
        }

        // For large clusters (> 16 backends), full O(N) scan causes CPU contention.
        // We use Power-of-K Choices (P2C / Sampled Candidates, K=4) with zero allocation
        // to achieve O(1) selection with near-optimal load distribution.
        let mut rng = rand::thread_rng();
        let mut min_score = f64::MAX;
        let mut best: Option<&Arc<Backend>> = None;
        let mut sampled_count = 0;
        let max_samples = 4.min(backends.len());
        let max_attempts = max_samples * 3; // Guard against sampling downed nodes

        for _ in 0..max_attempts {
            let idx = rng.gen_range(0..backends.len());
            let b = &backends[idx];
            if !b.is_available() {
                continue;
            }

            let score = self.calculate_score(b);
            if score < min_score - 1e-6 {
                min_score = score;
                best = Some(b);
            }

            sampled_count += 1;
            if sampled_count >= max_samples {
                break;
            }
        }

        if let Some(b) = best {
            return Some((*b).clone());
        }

        // Fallback: If random sampling failed to find healthy nodes, scan sequentially
        for b in backends {
            if b.is_available() {
                return Some(b.clone());
            }
        }

        None
    }

    fn algorithm(&self) -> AlgorithmType {
        AlgorithmType::Adaptive
    }
}
