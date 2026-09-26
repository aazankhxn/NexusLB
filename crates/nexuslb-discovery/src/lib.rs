pub mod dns;
pub mod file;
pub mod k8s;
pub mod manager;
pub mod traits;

pub use dns::DnsDiscovery;
pub use file::FileCatalogDiscovery;
pub use k8s::KubernetesEndpointsDiscovery;
pub use manager::DiscoveryManager;
pub use traits::{DiscoveredInstance, DiscoveryEvent, ServiceDiscoveryProvider};
