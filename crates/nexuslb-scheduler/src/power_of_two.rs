use crate::traits::{Scheduler, SelectionContext};
use nexuslb_core::backend::Backend;
use nexuslb_core::types::AlgorithmType;
use std::cell::Cell;
use std::sync::Arc;

thread_local! {
    static RNG_STATE: Cell<u64> = Cell::new({
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        if seed == 0 { 0x853c49e6748fea9b } else { seed }
    });
}

#[inline(always)]
fn fast_rand_range(upper_bound: usize) -> usize {
    if upper_bound <= 1 {
        return 0;
    }
    RNG_STATE.with(|cell| {
        let mut x = cell.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        cell.set(x);
        (x as usize) % upper_bound
    })
}

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
        let len = backends.len();
        if len == 0 {
            return None;
        }

        if len == 1 {
            return if backends[0].is_available() {
                Some(backends[0].clone())
            } else {
                None
            };
        }

        let idx1 = fast_rand_range(len);
        let mut idx2 = fast_rand_range(len);
        if idx1 == idx2 {
            idx2 = (idx1 + 1) % len;
        }

        let b1 = &backends[idx1];
        let b2 = &backends[idx2];
        let b1_avail = b1.is_available();
        let b2_avail = b2.is_available();

        if b1_avail && b2_avail {
            // Evaluate score: connections normalized by weight, plus EWMA latency in millis
            let score1 = (b1.active_connections() * 1000) / (b1.weight().max(1) as u64)
                + b1.stats().ewma_latency().as_millis() as u64;
            let score2 = (b2.active_connections() * 1000) / (b2.weight().max(1) as u64)
                + b2.stats().ewma_latency().as_millis() as u64;

            if score1 <= score2 {
                Some(b1.clone())
            } else {
                Some(b2.clone())
            }
        } else if b1_avail {
            Some(b1.clone())
        } else if b2_avail {
            Some(b2.clone())
        } else {
            // Fallback: bounded scan from randomized offset to eliminate cascade on index 0
            let start = fast_rand_range(len);
            for i in 0..len {
                let candidate = &backends[(start + i) % len];
                if candidate.is_available() {
                    return Some(candidate.clone());
                }
            }
            None
        }
    }

    fn algorithm(&self) -> AlgorithmType {
        AlgorithmType::PowerOfTwoChoices
    }
}
