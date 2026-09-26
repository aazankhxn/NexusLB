use crossbeam::queue::ArrayQueue;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub const DEFAULT_BUFFER_SIZE: usize = 16 * 1024; // 16KB optimal for HTTP / TCP streaming
pub const DEFAULT_POOL_CAPACITY: usize = 512;

/// Lock-free memory pool for reusable I/O buffers to eliminate hot-path allocations
#[derive(Clone)]
pub struct BufferPool {
    inner: Arc<BufferPoolInner>,
}

struct BufferPoolInner {
    queue: ArrayQueue<Vec<u8>>,
    buffer_size: usize,
}

impl BufferPool {
    pub fn new(capacity: usize, buffer_size: usize) -> Self {
        let queue = ArrayQueue::new(capacity);
        // Pre-warm the pool with a lean working set (64 buffers = 1MB)
        let prewarm_count = capacity.min(64);
        for _ in 0..prewarm_count {
            let _ = queue.push(vec![0u8; buffer_size]);
        }

        Self {
            inner: Arc::new(BufferPoolInner { queue, buffer_size }),
        }
    }

    pub fn acquire(&self) -> PooledBuffer {
        let mut buffer = self
            .inner
            .queue
            .pop()
            .unwrap_or_else(|| vec![0u8; self.inner.buffer_size]);

        buffer.resize(self.inner.buffer_size, 0);

        PooledBuffer {
            pool: Some(self.clone()),
            data: buffer,
        }
    }

    fn release(&self, mut buffer: Vec<u8>) {
        if buffer.capacity() < self.inner.buffer_size {
            buffer.reserve(self.inner.buffer_size - buffer.capacity());
        }
        buffer.clear();
        let _ = self.inner.queue.push(buffer);
    }
}

impl Default for BufferPool {
    fn default() -> Self {
        Self::new(DEFAULT_POOL_CAPACITY, DEFAULT_BUFFER_SIZE)
    }
}

/// RAII wrapper that returns the buffer to the pool upon drop
pub struct PooledBuffer {
    pool: Option<BufferPool>,
    data: Vec<u8>,
}

impl PooledBuffer {
    pub fn new_standalone(size: usize) -> Self {
        Self {
            pool: None,
            data: vec![0u8; size],
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

impl Deref for PooledBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.take() {
            let data = std::mem::take(&mut self.data);
            pool.release(data);
        }
    }
}
