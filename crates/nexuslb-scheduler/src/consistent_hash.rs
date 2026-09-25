use crate::traits::{Scheduler, SelectionContext};
use ahash::AHasher;
use nexuslb_core::backend::Backend;
use nexuslb_core::types::{AlgorithmType, BackendId};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::hash::Hasher;
use std::sync::Arc;

const VNODES_PER_WEIGHT: usize = 64;

#[derive(Clone)]
struct VirtualNode {
    hash: u64,
    backend_id: BackendId,
}

pub struct ConsistentHashScheduler {
    ring: RwLock<Vec<VirtualNode>>,
    cached_backend_ids: RwLock<Vec<BackendId>>,
}

impl ConsistentHashScheduler {
    pub fn new() -> Self {
        Self {
            ring: RwLock::new(Vec::new()),
            cached_backend_ids: RwLock::new(Vec::new()),
        }
    }

    fn rebuild_ring_if_needed(&self, backends: &[Arc<Backend>]) {
        let current_ids: Vec<BackendId> = backends.iter().map(|b| b.id()).collect();
        {
            let cached = self.cached_backend_ids.read();
            if *cached == current_ids {
                return;
            }
        }

        let mut ring = Vec::new();
        for b in backends {
            let num_vnodes = (b.weight() as usize).max(1) * VNODES_PER_WEIGHT;
            for v in 0..num_vnodes {
                let mut hasher = AHasher::default();
                hasher.write_u64(b.id().0);
                hasher.write_usize(v);
                let hash = hasher.finish();
                ring.push(VirtualNode {
                    hash,
                    backend_id: b.id(),
                });
            }
        }

        ring.sort_by_key(|node| node.hash);

        *self.ring.write() = ring;
        *self.cached_backend_ids.write() = current_ids;
    }
}

impl Default for ConsistentHashScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for ConsistentHashScheduler {
    fn select(&self, backends: &[Arc<Backend>], ctx: &SelectionContext) -> Option<Arc<Backend>> {
        if backends.is_empty() {
            return None;
        }

        self.rebuild_ring_if_needed(backends);

        let ring = self.ring.read();
        if ring.is_empty() {
            return None;
        }

        let mut hasher = AHasher::default();
        if let Some(key) = ctx.key {
            hasher.write(key.as_bytes());
        } else if let Some(ip) = ctx.client_ip {
            match ip {
                std::net::IpAddr::V4(v4) => hasher.write(&v4.octets()),
                std::net::IpAddr::V6(v6) => hasher.write(&v6.octets()),
            }
        } else {
            hasher.write_u64(0xfeedface);
        }
        let target_hash = hasher.finish();

        let backend_map: HashMap<BackendId, &Arc<Backend>> =
            backends.iter().map(|b| (b.id(), b)).collect();

        // Binary search for first node with hash >= target_hash
        let partition_idx = ring.partition_point(|node| node.hash < target_hash);
        let ring_len = ring.len();

        for i in 0..ring_len {
            let idx = (partition_idx + i) % ring_len;
            let candidate_id = ring[idx].backend_id;
            if let Some(b) = backend_map.get(&candidate_id) {
                if b.is_available() {
                    return Some((*b).clone());
                }
            }
        }

        None
    }

    fn algorithm(&self) -> AlgorithmType {
        AlgorithmType::ConsistentHash
    }
}
