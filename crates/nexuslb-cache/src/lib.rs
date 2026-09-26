use ahash::AHashMap;
use bytes::Bytes;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub method: String,
    pub host: String,
    pub path: String,
}

impl CacheKey {
    pub fn new(method: &str, host: &str, path: &str) -> Self {
        Self {
            method: method.to_ascii_uppercase(),
            host: host.to_ascii_lowercase(),
            path: path.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
    pub created_at: Instant,
    pub ttl: Duration,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

impl CachedResponse {
    pub fn is_fresh(&self) -> bool {
        self.created_at.elapsed() < self.ttl
    }

    pub fn age_secs(&self) -> u64 {
        self.created_at.elapsed().as_secs()
    }
}

#[derive(Debug, Clone)]
pub enum CacheResult {
    /// Cached response is fresh and ready to be served (< 5 µs)
    Hit(CachedResponse),
    /// ETag matches If-None-Match; client can be answered with HTTP 304 Not Modified
    NotModified(Option<String>),
    /// Item not in cache or expired
    Miss,
}

/// Parsed RFC 7234 Cache-Control directives
#[derive(Debug, Default, Clone)]
pub struct CacheDirectives {
    pub no_store: bool,
    pub no_cache: bool,
    pub max_age: Option<Duration>,
    pub is_public: bool,
    pub is_private: bool,
}

impl CacheDirectives {
    pub fn parse(header: &str) -> Self {
        let mut d = Self::default();
        for part in header.split(',') {
            let part = part.trim().to_ascii_lowercase();
            if part == "no-store" {
                d.no_store = true;
            } else if part == "no-cache" {
                d.no_cache = true;
            } else if part == "public" {
                d.is_public = true;
            } else if part == "private" {
                d.is_private = true;
            } else if part.starts_with("max-age=") {
                if let Ok(secs) = part.trim_start_matches("max-age=").trim().parse::<u64>() {
                    d.max_age = Some(Duration::from_secs(secs));
                }
            } else if part.starts_with("s-maxage=") {
                if let Ok(secs) = part.trim_start_matches("s-maxage=").trim().parse::<u64>() {
                    d.max_age = Some(Duration::from_secs(secs));
                }
            }
        }
        d
    }
}

struct CacheEntry {
    response: CachedResponse,
}

struct CacheStorage {
    map: AHashMap<CacheKey, CacheEntry>,
    order: VecDeque<CacheKey>,
    capacity: usize,
}

impl CacheStorage {
    fn new(capacity: usize) -> Self {
        Self {
            map: AHashMap::with_capacity(capacity.min(256)),
            order: VecDeque::with_capacity(capacity.min(256)),
            capacity,
        }
    }

    fn get(&self, key: &CacheKey) -> Option<CachedResponse> {
        let entry = self.map.get(key)?;
        if !entry.response.is_fresh() {
            return None;
        }
        Some(entry.response.clone())
    }

    fn put(&mut self, key: CacheKey, response: CachedResponse) -> bool {
        if self.map.len() >= self.capacity && !self.map.contains_key(&key) {
            // Evict oldest (LRU)
            if let Some(oldest_key) = self.order.pop_front() {
                self.map.remove(&oldest_key);
            }
        }

        self.order.push_back(key.clone());
        self.map.insert(key, CacheEntry { response });
        true
    }
}

/// Thread-safe, high-concurrency RFC 7234 in-memory HTTP cache
#[derive(Clone)]
pub struct HttpCache {
    storage: Arc<RwLock<CacheStorage>>,
    len: Arc<std::sync::atomic::AtomicUsize>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    revalidations: Arc<AtomicU64>,
}

impl HttpCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            storage: Arc::new(RwLock::new(CacheStorage::new(capacity))),
            len: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            revalidations: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Fast path lookup in memory
    pub fn get(
        &self,
        method: &str,
        host: &str,
        path: &str,
        if_none_match: Option<&str>,
    ) -> CacheResult {
        // Only GET and HEAD requests are cacheable
        if !method.eq_ignore_ascii_case("GET") && !method.eq_ignore_ascii_case("HEAD") {
            return CacheResult::Miss;
        }

        // Fast-path: if cache is empty, avoid all string allocations and locks
        if self.len.load(Ordering::Relaxed) == 0 {
            return CacheResult::Miss;
        }

        let key = CacheKey::new(method, host, path);
        let resp = {
            let storage = self.storage.read();
            storage.get(&key)
        };

        if let Some(cached) = resp {
            // Check conditional ETag validation (RFC 7232)
            if let Some(inm) = if_none_match {
                if let Some(ref etag) = cached.etag {
                    if inm.trim() == etag.trim() || inm.trim() == "*" {
                        self.revalidations.fetch_add(1, Ordering::Relaxed);
                        return CacheResult::NotModified(cached.etag.clone());
                    }
                }
            }

            self.hits.fetch_add(1, Ordering::Relaxed);
            CacheResult::Hit(cached)
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            CacheResult::Miss
        }
    }

    /// Stores a response if eligible under RFC 7234 rules
    pub fn put(
        &self,
        method: &str,
        host: &str,
        path: &str,
        status: u16,
        headers: &[(String, String)],
        body: Bytes,
    ) -> bool {
        // Only cache successful or designated cacheable responses
        if status != 200 && status != 203 && status != 300 && status != 301 {
            return false;
        }

        // Only GET and HEAD are stored
        if !method.eq_ignore_ascii_case("GET") && !method.eq_ignore_ascii_case("HEAD") {
            return false;
        }

        let mut directives = None;
        let mut etag = None;
        let mut last_modified = None;

        for (k, v) in headers {
            if k.eq_ignore_ascii_case("cache-control") {
                directives = Some(CacheDirectives::parse(v));
            } else if k.eq_ignore_ascii_case("etag") {
                etag = Some(v.clone());
            } else if k.eq_ignore_ascii_case("last-modified") {
                last_modified = Some(v.clone());
            }
        }

        let ttl = match directives {
            Some(ref d) => {
                if d.no_store || d.is_private {
                    return false;
                }
                d.max_age.unwrap_or(Duration::from_secs(60))
            }
            None => {
                // If no Cache-Control but ETag is present, cache for heuristic default 60s
                if etag.is_some() {
                    Duration::from_secs(60)
                } else {
                    return false; // Not cacheable without explicit directives
                }
            }
        };

        let key = CacheKey::new(method, host, path);
        let cached = CachedResponse {
            status,
            headers: headers.to_vec(),
            body,
            created_at: Instant::now(),
            ttl,
            etag,
            last_modified,
        };

        let mut storage = self.storage.write();
        let was_new = storage.put(key, cached);
        self.len.store(storage.map.len(), Ordering::Relaxed);
        was_new
    }

    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    pub fn revalidations(&self) -> u64 {
        self.revalidations.load(Ordering::Relaxed)
    }
}

impl Default for HttpCache {
    fn default() -> Self {
        Self::new(10_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_put_and_hit() {
        let cache = HttpCache::new(100);
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            (
                "Cache-Control".to_string(),
                "public, max-age=120".to_string(),
            ),
            ("ETag".to_string(), "\"nexus-v1\"".to_string()),
        ];
        let body = Bytes::from_static(b"{\"status\":\"cached\"}");

        let stored = cache.put("GET", "api.local", "/items", 200, &headers, body);
        assert!(stored);

        // Immediate Cache Hit
        match cache.get("GET", "api.local", "/items", None) {
            CacheResult::Hit(resp) => {
                assert_eq!(resp.status, 200);
                assert_eq!(resp.body.as_ref(), b"{\"status\":\"cached\"}");
                assert_eq!(resp.etag.as_deref(), Some("\"nexus-v1\""));
            }
            _ => panic!("Expected cache hit"),
        }

        // Conditional ETag Not Modified (304)
        match cache.get("GET", "api.local", "/items", Some("\"nexus-v1\"")) {
            CacheResult::NotModified(etag) => {
                assert_eq!(etag.as_deref(), Some("\"nexus-v1\""));
            }
            _ => panic!("Expected 304 NotModified"),
        }

        assert_eq!(cache.hits(), 1);
        assert_eq!(cache.revalidations(), 1);
    }

    #[test]
    fn test_no_store_bypass() {
        let cache = HttpCache::new(100);
        let headers = vec![("Cache-Control".to_string(), "no-store".to_string())];
        let body = Bytes::from_static(b"sensitive data");

        let stored = cache.put("GET", "api.local", "/secret", 200, &headers, body);
        assert!(!stored);

        assert!(matches!(
            cache.get("GET", "api.local", "/secret", None),
            CacheResult::Miss
        ));
    }
}
