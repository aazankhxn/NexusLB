use crate::traits::{DiscoveredInstance, ServiceDiscoveryProvider};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info};

pub struct DiscoveryManager {
    providers: Vec<Box<dyn ServiceDiscoveryProvider>>,
    cached_instances: Arc<RwLock<HashMap<SocketAddr, DiscoveredInstance>>>,
}

/// Maximum number of discovered instances cached to prevent memory exhaustion from rogue providers.
const MAX_CACHED_DISCOVERY_INSTANCES: usize = 10_000;

impl DiscoveryManager {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            cached_instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_provider(&mut self, provider: Box<dyn ServiceDiscoveryProvider>) {
        info!(
            provider = provider.name(),
            "Registered service discovery provider"
        );
        self.providers.push(provider);
    }

    /// Run a one-time synchronization pass across all providers
    pub async fn sync_once(&self) -> Vec<DiscoveredInstance> {
        let mut all_instances = Vec::new();

        for provider in &self.providers {
            match provider.discover().await {
                Ok(instances) => {
                    all_instances.extend(instances);
                }
                Err(e) => {
                    error!(provider = provider.name(), error = %e, "Service discovery refresh failed");
                }
            }
        }

        if all_instances.len() > MAX_CACHED_DISCOVERY_INSTANCES {
            tracing::warn!(
                total = all_instances.len(),
                max = MAX_CACHED_DISCOVERY_INSTANCES,
                "Discovered instance count exceeds safety limit, truncating to prevent memory exhaustion"
            );
            all_instances.truncate(MAX_CACHED_DISCOVERY_INSTANCES);
        }

        let mut cache = self.cached_instances.write().await;
        cache.clear();
        for inst in &all_instances {
            cache.insert(inst.address, inst.clone());
        }

        all_instances
    }

    /// Start a periodic discovery background loop
    pub fn start_periodic_sync(self: Arc<Self>, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let _ = self.sync_once().await;
            }
        })
    }
}

impl Default for DiscoveryManager {
    fn default() -> Self {
        Self::new()
    }
}
