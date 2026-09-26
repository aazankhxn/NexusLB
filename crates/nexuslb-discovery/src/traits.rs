use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredInstance {
    pub name: String,
    pub address: SocketAddr,
    pub weight: u32,
    pub pool: String,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryEvent {
    Added(DiscoveredInstance),
    Removed(SocketAddr),
    Updated(DiscoveredInstance),
}

#[async_trait]
pub trait ServiceDiscoveryProvider: Send + Sync {
    /// Return the friendly name of this discovery provider
    fn name(&self) -> &str;

    /// Query or refresh all currently available instances
    async fn discover(&self) -> anyhow::Result<Vec<DiscoveredInstance>>;
}
