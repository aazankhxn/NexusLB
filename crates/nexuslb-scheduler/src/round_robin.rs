use crate::traits::{Scheduler, SelectionContext};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::AlgorithmType;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct RoundRobinScheduler {
    index: AtomicUsize,
}

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self {
            index: AtomicUsize::new(0),
        }
    }
}

impl Default for RoundRobinScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for RoundRobinScheduler {
    fn select(&self, backends: &[Arc<Backend>], _ctx: &SelectionContext) -> Option<Arc<Backend>> {
        if backends.is_empty() {
            return None;
        }

        let len = backends.len();
        let start = self.index.fetch_add(1, Ordering::Relaxed);

        // Try to find the next available backend
        for i in 0..len {
            let idx = (start + i) % len;
            let b = &backends[idx];
            if b.is_available() {
                return Some(b.clone());
            }
        }

        None
    }

    fn algorithm(&self) -> AlgorithmType {
        AlgorithmType::RoundRobin
    }
}
