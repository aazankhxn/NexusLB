use crate::traits::{Scheduler, SelectionContext};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::AlgorithmType;
use rand::Rng;
use std::sync::Arc;

pub struct PowerOfTwoChoicesScheduler;

impl PowerOfTwoChoicesScheduler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PowerOfTwoChoicesScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for PowerOfTwoChoicesScheduler {
    fn select(&self, backends: &[Arc<Backend>], _ctx: &SelectionContext) -> Option<Arc<Backend>> {
        let available: Vec<&Arc<Backend>> = backends.iter().filter(|b| b.is_available()).collect();
        if available.is_empty() {
            return None;
        }

        if available.len() == 1 {
            return Some((*available[0]).clone());
        }

        let mut rng = rand::thread_rng();
        let idx1 = rng.gen_range(0..available.len());
        let mut idx2 = rng.gen_range(0..available.len());
        if idx1 == idx2 {
            idx2 = (idx1 + 1) % available.len();
        }

        let b1 = available[idx1];
        let b2 = available[idx2];

        // Evaluate score: connections normalized by weight, plus EWMA latency in millis
        let score1 = (b1.active_connections() * 1000) / (b1.weight().max(1) as u64)
            + b1.stats().ewma_latency().as_millis() as u64;
        let score2 = (b2.active_connections() * 1000) / (b2.weight().max(1) as u64)
            + b2.stats().ewma_latency().as_millis() as u64;

        if score1 <= score2 {
            Some((*b1).clone())
        } else {
            Some((*b2).clone())
        }
    }

    fn algorithm(&self) -> AlgorithmType {
        AlgorithmType::PowerOfTwoChoices
    }
}
