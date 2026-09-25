use nexuslb_core::backend::Backend;
use nexuslb_core::types::AlgorithmType;
use nexuslb_scheduler::factory::create_scheduler;
use nexuslb_scheduler::traits::{Scheduler, SelectionContext};
use std::sync::Arc;

#[derive(Clone)]
pub struct PoolGroup {
    pub name: String,
    pub backends: Vec<Arc<Backend>>,
    pub algorithm: AlgorithmType,
    pub scheduler: Arc<dyn Scheduler>,
}

impl PoolGroup {
    pub fn new(
        name: impl Into<String>,
        backends: Vec<Arc<Backend>>,
        algorithm: AlgorithmType,
    ) -> Self {
        let scheduler = create_scheduler(algorithm);
        Self {
            name: name.into(),
            backends,
            algorithm,
            scheduler,
        }
    }

    #[inline(always)]
    pub fn select(&self, ctx: &SelectionContext) -> Option<Arc<Backend>> {
        self.scheduler.select(&self.backends, ctx)
    }

    pub fn healthy_count(&self) -> usize {
        self.backends.iter().filter(|b| b.is_available()).count()
    }
}
