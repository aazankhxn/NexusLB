use crate::traits::{Scheduler, SelectionContext};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::AlgorithmType;
use std::sync::Arc;

pub struct LeastConnectionsScheduler;

impl LeastConnectionsScheduler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LeastConnectionsScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for LeastConnectionsScheduler {
    fn select(&self, backends: &[Arc<Backend>], _ctx: &SelectionContext) -> Option<Arc<Backend>> {
        let mut min_conn = u64::MAX;
        let mut best: Option<&Arc<Backend>> = None;

        for b in backends {
            if !b.is_available() {
                continue;
            }

            let conns = b.active_connections();
            // Normalize by weight: higher weight decreases effective connection pressure
            let weight = b.weight().max(1) as u64;
            let effective_conns = (conns * 100) / weight;

            if effective_conns < min_conn {
                min_conn = effective_conns;
                best = Some(b);
            }
        }

        best.cloned()
    }

    fn algorithm(&self) -> AlgorithmType {
        AlgorithmType::LeastConnections
    }
}
