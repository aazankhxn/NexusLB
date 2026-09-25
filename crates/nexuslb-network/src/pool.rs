use parking_lot::Mutex;
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
    idle: VecDeque<PooledConnection>,
    active_count: AtomicU64,
}

impl BackendPool {
    fn new() -> Self {
        Self {
            idle: VecDeque::new(),
            active_count: AtomicU64::new(0),
        }
    }
}

#[derive(Clone)]
pub struct ConnectionPool {
    pools: Arc<Mutex<std::collections::HashMap<BackendId, BackendPool>>>,
    config: ConnectionPoolConfig,
}

impl ConnectionPool {
    pub fn new(config: ConnectionPoolConfig) -> Self {
        Self {
            pools: Arc::new(Mutex::new(std::collections::HashMap::new())),
            config,
        }
    }

    pub async fn get_or_connect(&self, id: BackendId, addr: SocketAddr) -> Result<TcpStream> {
        // Try getting an idle connection from the pool
        {
            let mut pools = self.pools.lock();
            let backend_pool = pools.entry(id).or_insert_with(BackendPool::new);

            while let Some(conn) = backend_pool.idle.pop_front() {
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
        let mut pools = self.pools.lock();
        let backend_pool = pools.entry(id).or_insert_with(BackendPool::new);

        backend_pool.active_count.fetch_sub(1, Ordering::Relaxed);

        if backend_pool.idle.len() < self.config.max_idle_per_backend && Self::is_alive(&stream) {
            backend_pool.idle.push_back(PooledConnection {
                stream,
                created_at: Instant::now(),
                last_used: Instant::now(),
            });
            trace!(backend_id = %id, "Returned connection to pool");
        }
        // Excess connection or dead connection gets closed on drop
    }

    pub fn dec_active(&self, id: BackendId) {
        let pools = self.pools.lock();
        if let Some(pool) = pools.get(&id) {
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
