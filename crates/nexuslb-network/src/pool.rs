use ahash::AHashMap;
use parking_lot::{Mutex, RwLock};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tracing::trace;

use nexuslb_core::error::{NexusError, Result};
use nexuslb_core::types::BackendId;

pub struct PooledConnection {
    pub stream: TcpStream,
    pub created_at: Instant,
    pub last_used: Instant,
}

#[derive(Clone)]
pub struct ConnectionPoolConfig {
    pub max_idle_per_backend: usize,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub connect_timeout: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_idle_per_backend: 64,
            idle_timeout: Duration::from_secs(45),
            max_lifetime: Duration::from_secs(300),
            connect_timeout: Duration::from_secs(3),
        }
    }
}

struct BackendPool {
    idle: Mutex<VecDeque<PooledConnection>>,
    active_count: AtomicU64,
}

impl BackendPool {
    fn new() -> Self {
        Self {
            idle: Mutex::new(VecDeque::new()),
            active_count: AtomicU64::new(0),
        }
    }
}

#[derive(Clone)]
pub struct ConnectionPool {
    pools: Arc<RwLock<AHashMap<BackendId, Arc<BackendPool>>>>,
    config: ConnectionPoolConfig,
}

impl ConnectionPool {
    pub fn new(config: ConnectionPoolConfig) -> Self {
        Self {
            pools: Arc::new(RwLock::new(AHashMap::new())),
            config,
        }
    }

    #[inline(always)]
    fn get_or_create_backend_pool(&self, id: BackendId) -> Arc<BackendPool> {
        let read = self.pools.read();
        if let Some(pool) = read.get(&id) {
            return pool.clone();
        }
        drop(read);
        let mut write = self.pools.write();
        write
            .entry(id)
            .or_insert_with(|| Arc::new(BackendPool::new()))
            .clone()
    }

    pub async fn get_or_connect(&self, id: BackendId, addr: SocketAddr) -> Result<TcpStream> {
        let backend_pool = self.get_or_create_backend_pool(id);

        // Try getting an idle connection from the pool without blocking any other backends
        {
            let mut idle = backend_pool.idle.lock();
            while let Some(conn) = idle.pop_front() {
                let now = Instant::now();
                if now.duration_since(conn.last_used) > self.config.idle_timeout
                    || now.duration_since(conn.created_at) > self.config.max_lifetime
                {
                    // Expired connection, drop it
                    continue;
                }

                // Check connection health (non-blocking peek or ready check)
                if Self::is_alive(&conn.stream) {
                    backend_pool.active_count.fetch_add(1, Ordering::Relaxed);
                    trace!(backend_id = %id, "Reusing pooled connection");
                    return Ok(conn.stream);
                }
            }
            backend_pool.active_count.fetch_add(1, Ordering::Relaxed);
        }

        // Establish new connection if no valid pooled connection was found
        trace!(backend_id = %id, target = %addr, "Establishing new upstream connection");
        match tokio::time::timeout(self.config.connect_timeout, TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => {
                let _ = stream.set_nodelay(true);
                Ok(stream)
            }
            Ok(Err(e)) => {
                self.dec_active(id);
                Err(NexusError::Io(e))
            }
            Err(_) => {
                self.dec_active(id);
                Err(NexusError::BackendTimeout(
                    nexuslb_core::error::BackendAddressError { id, addr },
                ))
            }
        }
    }

    pub fn return_connection(&self, id: BackendId, stream: TcpStream) {
        let read = self.pools.read();
        let backend_pool = read.get(&id).cloned();
        drop(read);

        if let Some(backend_pool) = backend_pool {
            backend_pool.active_count.fetch_sub(1, Ordering::Relaxed);

            let mut idle = backend_pool.idle.lock();
            if idle.len() < self.config.max_idle_per_backend && Self::is_alive(&stream) {
                idle.push_back(PooledConnection {
                    stream,
                    created_at: Instant::now(),
                    last_used: Instant::now(),
                });
                trace!(backend_id = %id, "Returned connection to pool");
            }
        }
        // Excess connection or dead connection gets closed on drop
    }

    pub fn dec_active(&self, id: BackendId) {
        let read = self.pools.read();
        if let Some(pool) = read.get(&id) {
            pool.active_count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    fn is_alive(stream: &TcpStream) -> bool {
        // Quick socket check to verify socket hasn't had FIN or RST received
        let mut buf = [0u8; 1];
        match stream.try_read(&mut buf) {
            Ok(0) => false, // EOF / remote closed
            Ok(_) => false, // Unexpected unread data on idle connection (poisoned/desynced) -> drop
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => true, // Still alive and clean
            Err(_) => false,
        }
    }
}
