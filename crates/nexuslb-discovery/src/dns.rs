use crate::traits::{DiscoveredInstance, ServiceDiscoveryProvider};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::net::lookup_host;
use tracing::{debug, trace};

pub struct DnsDiscovery {
    name: String,
    domain: String,
    port: u16,
    pool: String,
    default_weight: u32,
}

impl DnsDiscovery {
    pub fn new(domain: impl Into<String>, port: u16, pool: impl Into<String>) -> Self {
        let domain = domain.into();
        Self {
            name: format!("dns:{}", domain),
            domain,
            port,
            pool: pool.into(),
            default_weight: 100,
        }
    }
}

#[async_trait]
impl ServiceDiscoveryProvider for DnsDiscovery {
    fn name(&self) -> &str {
        &self.name
    }

    async fn discover(&self) -> anyhow::Result<Vec<DiscoveredInstance>> {
        let host_port = format!("{}:{}", self.domain, self.port);
        trace!(domain = %self.domain, port = self.port, "Resolving DNS endpoints");

        let addrs = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            lookup_host(&host_port),
        )
        .await
        .map_err(|_| anyhow::anyhow!("DNS lookup timed out for {}", self.domain))??;
        let mut instances = Vec::new();

        for (idx, addr) in addrs.enumerate() {
            let instance_name = format!("{}-{}", self.domain, idx + 1);
            let mut tags = HashMap::new();
            tags.insert("provider".to_string(), "dns".to_string());
            tags.insert("domain".to_string(), self.domain.clone());

            instances.push(DiscoveredInstance {
                name: instance_name,
                address: addr,
                weight: self.default_weight,
                pool: self.pool.clone(),
                tags,
            });
        }

        debug!(domain = %self.domain, count = instances.len(), "DNS discovery resolved instances");
        Ok(instances)
    }
}
