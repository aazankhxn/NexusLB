use crate::traits::{Scheduler, SelectionContext};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::{AlgorithmType, BackendId};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// Smooth weighted round-robin scheduling (NGINX-style)
pub struct WeightedRoundRobinScheduler {
    state: Mutex<HashMap<BackendId, i64>>,
}

impl WeightedRoundRobinScheduler {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for WeightedRoundRobinScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for WeightedRoundRobinScheduler {
    fn select(&self, backends: &[Arc<Backend>], _ctx: &SelectionContext) -> Option<Arc<Backend>> {
        let available: Vec<&Arc<Backend>> = backends.iter().filter(|b| b.is_available()).collect();
        if available.is_empty() {
            return None;
        }

        if available.len() == 1 {
            return Some((*available[0]).clone());
        }

        let mut state = self.state.lock();
        let mut total_weight = 0i64;
        let mut best_backend: Option<&Arc<Backend>> = None;
        let mut max_weight = i64::MIN;

        for &b in &available {
            let id = b.id();
            let effective_weight = b.weight() as i64;
            total_weight += effective_weight;

            let cur = state.entry(id).or_insert(0);
            *cur += effective_weight;

            if *cur > max_weight {
                max_weight = *cur;
                best_backend = Some(b);
            }
        }

        if let Some(best) = best_backend {
            if let Some(cur) = state.get_mut(&best.id()) {
                *cur -= total_weight;
            }
            return Some((*best).clone());
        }

        None
    }

    fn algorithm(&self) -> AlgorithmType {
        AlgorithmType::WeightedRoundRobin
    }
}
