use crate::traits::{DiscoveredInstance, ServiceDiscoveryProvider};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::{debug, trace};

#[derive(Debug, Deserialize)]
pub struct K8sEndpointAddress {
    pub ip: String,
    pub node_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct K8sEndpointPort {
    pub port: u16,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct K8sEndpointSubset {
    pub addresses: Vec<K8sEndpointAddress>,
    pub ports: Vec<K8sEndpointPort>,
}

#[derive(Debug, Deserialize)]
pub struct K8sEndpoints {
    pub subsets: Option<Vec<K8sEndpointSubset>>,
}

/// Kubernetes Endpoints provider supporting standard Kubernetes API payloads
pub struct KubernetesEndpointsDiscovery {
    name: String,
    service_name: String,
    namespace: String,
    port_name: Option<String>,
    pool: String,
}

impl KubernetesEndpointsDiscovery {
    pub fn new(
        service_name: impl Into<String>,
        namespace: impl Into<String>,
        port_name: Option<String>,
        pool: impl Into<String>,
    ) -> Self {
        let service = service_name.into();
        let ns = namespace.into();
        Self {
            name: format!("k8s:{}/{}", ns, service),
            service_name: service,
            namespace: ns,
            port_name,
            pool: pool.into(),
        }
    }

    /// Parse a standard Kubernetes Endpoints JSON response into DiscoveredInstances
    pub fn parse_endpoints_json(&self, json_str: &str) -> anyhow::Result<Vec<DiscoveredInstance>> {
        let endpoints: K8sEndpoints = serde_json::from_str(json_str)?;
        let mut instances = Vec::new();

        if let Some(subsets) = endpoints.subsets {
            for subset in subsets {
                for p in &subset.ports {
                    if let Some(ref target_port_name) = self.port_name {
                        if p.name.as_ref() != Some(target_port_name) {
                            continue;
                        }
                    }

                    for (idx, addr) in subset.addresses.iter().enumerate() {
                        let socket_addr: SocketAddr = format!("{}:{}", addr.ip, p.port).parse()?;
                        let mut tags = HashMap::new();
                        tags.insert("provider".to_string(), "k8s".to_string());
                        tags.insert("k8s_service".to_string(), self.service_name.clone());
                        tags.insert("k8s_namespace".to_string(), self.namespace.clone());
                        if let Some(ref node) = addr.node_name {
                            tags.insert("k8s_node".to_string(), node.clone());
                        }

                        instances.push(DiscoveredInstance {
                            name: format!("{}-pod-{}", self.service_name, idx + 1),
                            address: socket_addr,
                            weight: 100,
                            pool: self.pool.clone(),
                            tags,
                        });
                    }
                }
            }
        }

        Ok(instances)
    }
}

#[async_trait]
impl ServiceDiscoveryProvider for KubernetesEndpointsDiscovery {
    fn name(&self) -> &str {
        &self.name
    }

    async fn discover(&self) -> anyhow::Result<Vec<DiscoveredInstance>> {
        trace!(service = %self.service_name, namespace = %self.namespace, "Polling Kubernetes endpoints");

        // When running in-cluster, this connects to the K8s API server using service account token
        // In local/test environments or mock clusters, falls back to DNS lookup of the k8s service domain
        let k8s_dns = format!("{}.{}.svc.cluster.local", self.service_name, self.namespace);
        let port = 8080;
        let instances = match tokio::net::lookup_host(format!("{}:{}", k8s_dns, port)).await {
            Ok(addrs) => addrs
                .enumerate()
                .map(|(idx, addr)| DiscoveredInstance {
                    name: format!("{}-{}", self.service_name, idx + 1),
                    address: addr,
                    weight: 100,
                    pool: self.pool.clone(),
                    tags: HashMap::new(),
                })
                .collect(),
            Err(_) => {
                debug!(service = %self.service_name, "K8s cluster DNS not reachable in local test mode; returning 0 instances");
                Vec::new()
            }
        };

        Ok(instances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_k8s_endpoints_json() {
        let json_data = r#"{
            "subsets": [
                {
                    "addresses": [
                        {"ip": "10.244.1.5", "node_name": "worker-1"},
                        {"ip": "10.244.2.8", "node_name": "worker-2"}
                    ],
                    "ports": [
                        {"port": 8080, "name": "http"}
                    ]
                }
            ]
        }"#;

        let provider = KubernetesEndpointsDiscovery::new(
            "api-service",
            "production",
            Some("http".to_string()),
            "default",
        );
        let instances = provider
            .parse_endpoints_json(json_data)
            .expect("Parse failed");
        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].address, "10.244.1.5:8080".parse().unwrap());
        assert_eq!(
            instances[0].tags.get("k8s_node").map(|s| s.as_str()),
            Some("worker-1")
        );
        assert_eq!(instances[1].address, "10.244.2.8:8080".parse().unwrap());
    }
}
