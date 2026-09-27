use crate::traits::{DiscoveredInstance, ServiceDiscoveryProvider};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tracing::{debug, trace};

#[derive(Debug, Deserialize)]
struct CatalogItem {
    pub name: String,
    pub address: SocketAddr,
    #[serde(default = "default_weight")]
    pub weight: u32,
    #[serde(default = "default_pool")]
    pub pool: String,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

fn default_weight() -> u32 {
    100
}

fn default_pool() -> String {
    "default".to_string()
}

pub struct FileCatalogDiscovery {
    name: String,
    path: PathBuf,
}

impl FileCatalogDiscovery {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let p = path.as_ref().to_path_buf();
        Self {
            name: format!("file:{}", p.display()),
            path: p,
        }
    }
}

#[async_trait]
impl ServiceDiscoveryProvider for FileCatalogDiscovery {
    fn name(&self) -> &str {
        &self.name
    }

    async fn discover(&self) -> anyhow::Result<Vec<DiscoveredInstance>> {
        trace!(path = %self.path.display(), "Reading discovery catalog file");
        const MAX_CATALOG_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB
        let metadata = tokio::fs::metadata(&self.path).await?;
        if !metadata.is_file() {
            anyhow::bail!(
                "Discovery catalog path {} is not a regular file (special character devices and pipes are not permitted)",
                self.path.display()
            );
        }
        if metadata.len() > MAX_CATALOG_FILE_SIZE {
            anyhow::bail!(
                "Discovery catalog file {} exceeds maximum permitted size ({} bytes > 10MB)",
                self.path.display(),
                metadata.len()
            );
        }

        use tokio::io::AsyncReadExt;
        let file = tokio::fs::File::open(&self.path).await?;
        let mut content = String::new();
        let mut reader = file.take(MAX_CATALOG_FILE_SIZE + 1);
        reader.read_to_string(&mut content).await?;
        if content.len() as u64 > MAX_CATALOG_FILE_SIZE {
            anyhow::bail!(
                "Discovery catalog file {} exceeded maximum permitted size (10MB)",
                self.path.display()
            );
        }

        let items: Vec<CatalogItem> = if self.path.extension().and_then(|e| e.to_str())
            == Some("yaml")
            || self.path.extension().and_then(|e| e.to_str()) == Some("yml")
        {
            serde_yaml::from_str(&content)?
        } else {
            serde_json::from_str(&content)?
        };

        let instances: Vec<DiscoveredInstance> = items
            .into_iter()
            .map(|item| DiscoveredInstance {
                name: item.name,
                address: item.address,
                weight: item.weight,
                pool: item.pool,
                tags: item.tags,
            })
            .collect();

        debug!(path = %self.path.display(), count = instances.len(), "File catalog discovery loaded instances");
        Ok(instances)
    }
}
