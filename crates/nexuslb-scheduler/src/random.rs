use crate::traits::{Scheduler, SelectionContext};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::AlgorithmType;
use rand::Rng;
use std::sync::Arc;

pub struct RandomScheduler;

impl RandomScheduler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RandomScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for RandomScheduler {
    fn select(&self, backends: &[Arc<Backend>], _ctx: &SelectionContext) -> Option<Arc<Backend>> {
        let available: Vec<&Arc<Backend>> = backends.iter().filter(|b| b.is_available()).collect();
        if available.is_empty() {
            return None;
        }

        let total_weight: u32 = available.iter().map(|b| b.weight()).sum();
        if total_weight == 0 {
            return Some((*available[0]).clone());
        }

        let mut rng = rand::thread_rng();
        let mut target = rng.gen_range(0..total_weight);

        for b in &available {
            let w = b.weight();
            if target < w {
                return Some((*b).clone());
            }
            target -= w;
        }

        Some((*available.last().unwrap()).clone())
    }

    fn algorithm(&self) -> AlgorithmType {
        AlgorithmType::Random
    }
}
