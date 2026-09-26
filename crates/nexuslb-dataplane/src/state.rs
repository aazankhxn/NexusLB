use arc_swap::ArcSwap;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

use nexuslb_network::{BufferPool, ConnectionPool};
use nexuslb_proxy::rate_limiter::RateLimiter;
use nexuslb_proxy::retry::RetryPolicy;
use nexuslb_router::Router;

/// Read-mostly immutable state snapshot for the dataplane
pub struct DataplaneState {
    pub router: Arc<Router>,
    pub rate_limiter: Arc<RateLimiter>,
    pub conn_pool: ConnectionPool,
    pub buffer_pool: BufferPool,
    pub retry_policy: RetryPolicy,
    pub tls_acceptor: Option<TlsAcceptor>,
    pub http_cache: Arc<nexuslb_cache::HttpCache>,
}

/// Thread-safe lock-free holder for atomic zero-downtime reconfiguration
pub struct SharedDataplaneState {
    inner: ArcSwap<DataplaneState>,
}

impl SharedDataplaneState {
    pub fn new(state: DataplaneState) -> Self {
        Self {
            inner: ArcSwap::new(Arc::new(state)),
        }
    }

    #[inline(always)]
    pub fn load(&self) -> arc_swap::Guard<Arc<DataplaneState>> {
        self.inner.load()
    }

    pub fn swap(&self, new_state: DataplaneState) {
        self.inner.store(Arc::new(new_state));
    }
}
