use crate::traits::{Scheduler, SelectionContext};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::AlgorithmType;
use std::sync::Arc;
use std::time::Duration;

pub struct EwmaLatencyScheduler;

impl EwmaLatencyScheduler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EwmaLatencyScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for EwmaLatencyScheduler {
    fn select(&self, backends: &[Arc<Backend>], _ctx: &SelectionContext) -> Option<Arc<Backend>> {
        let mut min_ewma = Duration::MAX;
        let mut best: Option<&Arc<Backend>> = None;

        for b in backends {
            if !b.is_available() {
                continue;
            }

            let ewma = b.stats().ewma_latency();
            if ewma < min_ewma {
                min_ewma = ewma;
                best = Some(b);
            }
        }

        best.cloned()
    }

    fn algorithm(&self) -> AlgorithmType {
        AlgorithmType::EwmaLatency
    }
}
