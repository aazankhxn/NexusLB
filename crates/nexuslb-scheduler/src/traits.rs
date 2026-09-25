use nexuslb_core::backend::Backend;
use nexuslb_core::types::AlgorithmType;
use std::net::IpAddr;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct SelectionContext<'a> {
    pub client_ip: Option<IpAddr>,
    pub key: Option<&'a str>,
}

impl<'a> SelectionContext<'a> {
    pub fn with_ip(client_ip: IpAddr) -> Self {
        Self {
            client_ip: Some(client_ip),
            key: None,
        }
    }

    pub fn with_key(key: &'a str) -> Self {
        Self {
            client_ip: None,
            key: Some(key),
        }
    }
}

pub trait Scheduler: Send + Sync {
    /// Select the next optimal backend from the available pool
    fn select(&self, backends: &[Arc<Backend>], ctx: &SelectionContext) -> Option<Arc<Backend>>;

    /// Return the scheduler algorithm type
    fn algorithm(&self) -> AlgorithmType;
}
