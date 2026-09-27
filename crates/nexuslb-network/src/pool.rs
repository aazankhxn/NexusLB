use ahash::AHashMap;
use crossbeam::queue::ArrayQueue;
use parking_lot::RwLock;
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
            max_idle_per_backend: 256,
            idle_timeout: Duration::from_secs(45),
            max_lifetime: Duration::from_secs(300),
            connect_timeout: Duration::from_secs(3),
        }
    }
}

/// Lock-free idle connection slot using ArrayQueue for zero-contention acquire/return
struct BackendPool {
    idle: ArrayQueue<PooledConnection>,
    active_count: AtomicU64,
}

impl BackendPool {
    fn new(capacity: usize) -> Self {
        Self {
            idle: ArrayQueue::new(capacity.max(8)),
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
        let sweep_interval = (config.idle_timeout / 2).max(Duration::from_secs(5));
        let pool = Self {
            pools: Arc::new(RwLock::new(AHashMap::new())),
            config,
        };

        // If running inside Tokio runtime, spawn periodic idle eviction.
        // Use a Weak reference so the background task terminates cleanly when
        // the ConnectionPool is dropped, preventing an Arc reference cycle and task leak.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let weak_pools = Arc::downgrade(&pool.pools);
            let config = pool.config.clone();
            handle.spawn(async move {
                let mut ticker = tokio::time::interval(sweep_interval);
                loop {
                    ticker.tick().await;
                    if let Some(pools) = weak_pools.upgrade() {
                        Self::evict_idle_internal(&pools, &config);
                    } else {
                        break;
                    }
                }
            });
        }

        pool
    }

    #[inline(always)]
    fn get_or_create_backend_pool(&self, id: BackendId) -> Arc<BackendPool> {
        // Fast path: read-only lock (zero contention when pool exists, which is >99.99% of calls)
        {
            let read = self.pools.read();
            if let Some(pool) = read.get(&id) {
                return pool.clone();
            }
        }
        // Slow path: write lock only on first-ever access per backend
        let mut write = self.pools.write();
        write
            .entry(id)
            .or_insert_with(|| Arc::new(BackendPool::new(self.config.max_idle_per_backend)))
            .clone()
    }

    pub async fn get_or_connect(&self, id: BackendId, addr: SocketAddr) -> Result<TcpStream> {
        let backend_pool = self.get_or_create_backend_pool(id);

        // Lock-free pop from ArrayQueue — no mutex contention
        let now = Instant::now();
        while let Some(conn) = backend_pool.idle.pop() {
            let idle_dur = now.duration_since(conn.last_used);
            if idle_dur > self.config.idle_timeout
                || now.duration_since(conn.created_at) > self.config.max_lifetime
            {
                // Expired connection, drop it and try next
                continue;
            }

            // If recently used (<100ms), skip the syscall liveness check
            let is_alive = if idle_dur < Duration::from_millis(100) {
                true
            } else {
                Self::is_alive(&conn.stream)
            };

            if is_alive {
                backend_pool.active_count.fetch_add(1, Ordering::Relaxed);
                trace!(backend_id = %id, "Reusing pooled connection");
                return Ok(conn.stream);
            }
            // Dead connection — drop and try next
        }

        backend_pool.active_count.fetch_add(1, Ordering::Relaxed);

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
        // Fast path: read lock to get pool reference
        let backend_pool = {
            let read = self.pools.read();
            read.get(&id).cloned()
        };

        if let Some(backend_pool) = backend_pool {
            backend_pool.active_count.fetch_sub(1, Ordering::Relaxed);

            let now = Instant::now();
            // Lock-free push to ArrayQueue — no mutex contention
            let _ = backend_pool.idle.push(PooledConnection {
                stream,
                created_at: now,
                last_used: now,
            });
            // If queue is full, push returns Err and the connection is silently dropped
            trace!(backend_id = %id, "Returned connection to pool");
        }
        // Excess connection gets closed on drop
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

    /// Periodic maintenance sweep: drain expired and closed connections across all backend pools,
    /// and prune empty/unused backend pool entries to prevent resource leakage.
    pub fn evict_idle(&self) {
        Self::evict_idle_internal(&self.pools, &self.config);
    }

    fn evict_idle_internal(
        pools: &Arc<RwLock<AHashMap<BackendId, Arc<BackendPool>>>>,
        config: &ConnectionPoolConfig,
    ) {
        let now = Instant::now();
        let pools_snapshot: Vec<(BackendId, Arc<BackendPool>)> = {
            let read = pools.read();
            read.iter().map(|(&k, v)| (k, v.clone())).collect()
        };

        let mut empty_pools = Vec::new();

        for (id, pool) in pools_snapshot {
            let mut retained = Vec::new();
            while let Some(conn) = pool.idle.pop() {
                let idle_dur = now.duration_since(conn.last_used);
                let life_dur = now.duration_since(conn.created_at);
                if idle_dur <= config.idle_timeout
                    && life_dur <= config.max_lifetime
                    && Self::is_alive(&conn.stream)
                {
                    retained.push(conn);
                }
            }

            let is_empty = retained.is_empty() && pool.active_count.load(Ordering::Relaxed) == 0;
            for conn in retained {
                let _ = pool.idle.push(conn);
            }

            if is_empty {
                empty_pools.push(id);
            }
        }

        if !empty_pools.is_empty() {
            let mut write = pools.write();
            for id in empty_pools {
                if let Some(pool) = write.get(&id) {
                    if pool.idle.is_empty() && pool.active_count.load(Ordering::Relaxed) == 0 {
                        write.remove(&id);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool_evict_idle() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Background accept task
        tokio::spawn(async move { while let Ok((_, _)) = listener.accept().await {} });

        let config = ConnectionPoolConfig {
            idle_timeout: Duration::from_millis(50),
            ..Default::default()
        };
        let pool = ConnectionPool::new(config);

        let id = BackendId::new(101);
        let stream = pool.get_or_connect(id, addr).await.unwrap();
        pool.return_connection(id, stream);

        assert_eq!(pool.pools.read().len(), 1);

        // Sleep until idle timeout expires
        tokio::time::sleep(Duration::from_millis(70)).await;

        pool.evict_idle();

        // Expired connection drained and empty pool pruned
        assert_eq!(pool.pools.read().len(), 0);
    }
}
