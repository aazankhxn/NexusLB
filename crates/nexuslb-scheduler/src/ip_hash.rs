use crate::traits::{Scheduler, SelectionContext};
use ahash::AHasher;
use nexuslb_core::backend::Backend;
use nexuslb_core::types::AlgorithmType;
use std::hash::Hasher;
use std::sync::Arc;

pub struct IpHashScheduler;

impl IpHashScheduler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for IpHashScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for IpHashScheduler {
    fn select(&self, backends: &[Arc<Backend>], ctx: &SelectionContext) -> Option<Arc<Backend>> {
        let available: Vec<&Arc<Backend>> = backends.iter().filter(|b| b.is_available()).collect();
        if available.is_empty() {
            return None;
        }

        let mut hasher = AHasher::default();
        if let Some(ip) = ctx.client_ip {
            match ip {
                std::net::IpAddr::V4(v4) => hasher.write(&v4.octets()),
                std::net::IpAddr::V6(v6) => hasher.write(&v6.octets()),
            }
        } else if let Some(key) = ctx.key {
            hasher.write(key.as_bytes());
        } else {
            hasher.write_u64(0x9e3779b97f4a7c15);
        }

        let hash = hasher.finish() as usize;
        let index = hash % available.len();
        Some((*available[index]).clone())
    }

    fn algorithm(&self) -> AlgorithmType {
        AlgorithmType::IpHash
    }
}
