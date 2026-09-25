use crate::traits::{Scheduler, SelectionContext};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::AlgorithmType;
use std::sync::Arc;
use std::time::Duration;

pub struct LeastLatencyScheduler;

impl LeastLatencyScheduler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LeastLatencyScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for LeastLatencyScheduler {
    fn select(&self, backends: &[Arc<Backend>], _ctx: &SelectionContext) -> Option<Arc<Backend>> {
        let mut min_latency = Duration::MAX;
        let mut best: Option<&Arc<Backend>> = None;

        for b in backends {
            if !b.is_available() {
                continue;
            }

            let lat = b.stats().average_latency();
            if lat < min_latency {
                min_latency = lat;
                best = Some(b);
            }
        }

        best.cloned()
    }

    fn algorithm(&self) -> AlgorithmType {
        AlgorithmType::LeastLatency
    }
}
