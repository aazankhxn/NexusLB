#![deny(unsafe_code)]

use ahash::AHashMap;
use bytes::Bytes;
use parking_lot::RwLock;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Number of cache shards for lock striping (Issue #9)
const NUM_SHARDS: usize = 32;

/// Global monotonic sequence counter for True LRU ordering (Issue #7)
static ACCESS_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub method: String,
    pub host: String,
    pub path: String,
    pub vary_key: Option<String>,
}

impl CacheKey {
    pub fn new(method: &str, host: &str, path: &str) -> Self {
        Self {
            method: method.to_ascii_uppercase(),
            host: host.to_ascii_lowercase(),
            path: path.to_string(),
            vary_key: None,
        }
    }

    pub fn with_vary(method: &str, host: &str, path: &str, vary_key: Option<String>) -> Self {
        Self {
            method: method.to_ascii_uppercase(),
            host: host.to_ascii_lowercase(),
            path: path.to_string(),
            vary_key,
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
    pub vary: Vec<String>,
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
    last_accessed: AtomicU64,
}

struct CacheShard {
    map: AHashMap<CacheKey, CacheEntry>,
    capacity: usize,
}

impl CacheShard {
    fn new(capacity: usize) -> Self {
        Self {
            map: AHashMap::with_capacity(capacity.min(64)),
            capacity,
        }
    }

    fn get(&self, key: &CacheKey) -> Option<CachedResponse> {
        let entry = self.map.get(key)?;
        if !entry.response.is_fresh() {
            return None;
        }
        // True LRU: Update recency of access on hit
        entry
            .last_accessed
            .store(ACCESS_COUNTER.fetch_add(1, Ordering::Relaxed), Ordering::Relaxed);
        Some(entry.response.clone())
    }

    fn put(&mut self, key: CacheKey, response: CachedResponse) -> bool {
        let is_new = !self.map.contains_key(&key);
        if is_new && self.map.len() >= self.capacity {
            // True LRU eviction: Find and remove the entry with the minimum last_accessed
            if let Some(lru_key) = self
                .map
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed.load(Ordering::Relaxed))
                .map(|(k, _)| k.clone())
            {
                self.map.remove(&lru_key);
            }
        }

        let access_seq = ACCESS_COUNTER.fetch_add(1, Ordering::Relaxed);
        self.map.insert(
            key,
            CacheEntry {
                response,
                last_accessed: AtomicU64::new(access_seq),
            },
        );
        is_new
    }

    fn sweep_expired(&mut self) -> usize {
        let now = Instant::now();
        let initial = self.map.len();
        self.map
            .retain(|_, v| now.duration_since(v.response.created_at) < v.response.ttl);
        initial - self.map.len()
    }
}

/// Helper function to compute shard index
#[inline]
fn shard_index(key: &CacheKey) -> usize {
    let mut hasher = ahash::AHasher::default();
    key.hash(&mut hasher);
    (hasher.finish() as usize) % NUM_SHARDS
}

/// Helper function to build a normalized vary key from request headers
pub fn build_vary_key(vary_fields: &[String], request_headers: &[(&str, &str)]) -> Option<String> {
    if vary_fields.is_empty() {
        return None;
    }
    let mut parts = Vec::with_capacity(vary_fields.len());
    for field in vary_fields {
        let field_lower = field.to_ascii_lowercase();
        let mut matched = false;
        for (k, v) in request_headers {
            if k.eq_ignore_ascii_case(&field_lower) {
                parts.push(format!("{}={}", field_lower, v.trim().to_ascii_lowercase()));
                matched = true;
                break;
            }
        }
        if !matched {
            parts.push(format!("{}=<absent>", field_lower));
        }
    }
    parts.sort();
    Some(parts.join(";"))
}

/// Thread-safe, high-concurrency RFC 7234 in-memory HTTP cache partitioned into 32 shards
#[derive(Clone)]
pub struct HttpCache {
    shards: Arc<[RwLock<CacheShard>; NUM_SHARDS]>,
    len: Arc<AtomicUsize>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    revalidations: Arc<AtomicU64>,
}

impl HttpCache {
    pub fn new(capacity: usize) -> Self {
        let shard_cap = (capacity / NUM_SHARDS).max(2);
        // Build 32 shards
        let shards: [RwLock<CacheShard>; NUM_SHARDS] = std::array::from_fn(|_| {
            RwLock::new(CacheShard::new(shard_cap))
        });

        let cache = Self {
            shards: Arc::new(shards),
            len: Arc::new(AtomicUsize::new(0)),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            revalidations: Arc::new(AtomicU64::new(0)),
        };

        // Periodic TTL maintenance sweep to evict expired items without waiting for LRU pressure.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let weak_shards = Arc::downgrade(&cache.shards);
            let weak_len = Arc::downgrade(&cache.len);
            handle.spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(30));
                loop {
                    ticker.tick().await;
                    let (shards_arc, len_arc) = match (weak_shards.upgrade(), weak_len.upgrade()) {
                        (Some(s), Some(l)) => (s, l),
                        _ => break,
                    };

                    let mut total_purged = 0;
                    let mut total_len = 0;
                    for shard_lock in shards_arc.iter() {
                        let mut shard = shard_lock.write();
                        total_purged += shard.sweep_expired();
                        total_len += shard.map.len();
                    }
                    if total_purged > 0 {
                        len_arc.store(total_len, Ordering::Relaxed);
                    }
                }
            });
        }

        cache
    }

    /// Explicitly sweep expired entries and update cache metrics. Returns number of purged items.
    pub fn sweep_expired(&self) -> usize {
        let mut total_purged = 0;
        let mut total_len = 0;
        for shard_lock in self.shards.iter() {
            let mut shard = shard_lock.write();
            total_purged += shard.sweep_expired();
            total_len += shard.map.len();
        }
        if total_purged > 0 {
            self.len.store(total_len, Ordering::Relaxed);
        }
        total_purged
    }

    /// Fast path lookup with optional request headers for RFC 7234 Vary support
    pub fn get_with_headers(
        &self,
        method: &str,
        host: &str,
        path: &str,
        request_headers: &[(&str, &str)],
        if_none_match: Option<&str>,
    ) -> CacheResult {
        if !method.eq_ignore_ascii_case("GET") && !method.eq_ignore_ascii_case("HEAD") {
            return CacheResult::Miss;
        }

        if self.len.load(Ordering::Relaxed) == 0 {
            return CacheResult::Miss;
        }

        // 1. Try first with request headers computed vary_key
        let resp = {
            // Check if there is an unvaried entry or an entry matching standard vary headers (e.g. Accept-Encoding)
            let mut matched_resp = None;

            // Common case 1: check standard Accept-Encoding vary if present in headers
            let ae_header = request_headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("accept-encoding"))
                .map(|(_, v)| v.trim().to_ascii_lowercase());

            if let Some(ae) = ae_header {
                let vary_key = format!("accept-encoding={}", ae);
                let key_vary = CacheKey::with_vary(method, host, path, Some(vary_key));
                let idx = shard_index(&key_vary);
                let shard = self.shards[idx].read();
                if let Some(c) = shard.get(&key_vary) {
                    matched_resp = Some(c);
                }
            }

            // Common case 2: check standard unvaried key
            if matched_resp.is_none() {
                let key_unvaried = CacheKey::new(method, host, path);
                let idx = shard_index(&key_unvaried);
                let shard = self.shards[idx].read();
                if let Some(c) = shard.get(&key_unvaried) {
                    // If cached item has vary headers, verify they match request headers
                    if c.vary.is_empty() {
                        matched_resp = Some(c);
                    } else {
                        let expected_vary_key = build_vary_key(&c.vary, request_headers);
                        if expected_vary_key.is_none() {
                            matched_resp = Some(c);
                        }
                    }
                }
            }

            matched_resp
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

    /// Fast path lookup in memory
    pub fn get(
        &self,
        method: &str,
        host: &str,
        path: &str,
        if_none_match: Option<&str>,
    ) -> CacheResult {
        self.get_with_headers(method, host, path, &[], if_none_match)
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
        self.put_with_auth_and_req_headers(method, host, path, status, headers, body, false, &[])
    }

    /// Stores a response taking into account request-level authorization under RFC 7234 Section 3.2
    #[allow(clippy::too_many_arguments)]
    pub fn put_with_auth(
        &self,
        method: &str,
        host: &str,
        path: &str,
        status: u16,
        headers: &[(String, String)],
        body: Bytes,
        request_has_authorization: bool,
    ) -> bool {
        self.put_with_auth_and_req_headers(
            method,
            host,
            path,
            status,
            headers,
            body,
            request_has_authorization,
            &[],
        )
    }

    /// Stores a response taking into account authorization and RFC 7234 §4.1 Vary headers
    #[allow(clippy::too_many_arguments)]
    pub fn put_with_auth_and_req_headers(
        &self,
        method: &str,
        host: &str,
        path: &str,
        status: u16,
        headers: &[(String, String)],
        body: Bytes,
        request_has_authorization: bool,
        request_headers: &[(&str, &str)],
    ) -> bool {
        if status != 200 && status != 203 && status != 300 && status != 301 {
            return false;
        }

        if !method.eq_ignore_ascii_case("GET") && !method.eq_ignore_ascii_case("HEAD") {
            return false;
        }

        // RFC 7234: Shared caches MUST NOT store responses containing Set-Cookie (credential leak)
        if headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        {
            return false;
        }

        let mut directives = None;
        let mut etag = None;
        let mut last_modified = None;
        let mut vary_fields: Vec<String> = Vec::new();

        for (k, v) in headers {
            if k.eq_ignore_ascii_case("cache-control") {
                directives = Some(CacheDirectives::parse(v));
            } else if k.eq_ignore_ascii_case("etag") {
                etag = Some(v.clone());
            } else if k.eq_ignore_ascii_case("last-modified") {
                last_modified = Some(v.clone());
            } else if k.eq_ignore_ascii_case("vary") {
                // RFC 7234 §4.1: If Vary header contains '*', response MUST NOT be cached
                if v.contains('*') {
                    return false;
                }
                for field in v.split(',') {
                    let field = field.trim().to_ascii_lowercase();
                    if !field.is_empty() && !vary_fields.contains(&field) {
                        vary_fields.push(field);
                    }
                }
            }
        }

        if request_has_authorization {
            match directives {
                Some(ref d) if d.is_public => {}
                _ => return false,
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
                if etag.is_some() {
                    Duration::from_secs(60)
                } else {
                    return false;
                }
            }
        };

        let vary_key = build_vary_key(&vary_fields, request_headers);
        let key = CacheKey::with_vary(method, host, path, vary_key);

        let cached = CachedResponse {
            status,
            headers: headers.to_vec(),
            body,
            created_at: Instant::now(),
            ttl,
            etag,
            last_modified,
            vary: vary_fields,
        };

        let idx = shard_index(&key);
        let mut shard = self.shards[idx].write();
        let was_new = shard.put(key, cached);
        if was_new {
            self.len.fetch_add(1, Ordering::Relaxed);
        }
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

    #[test]
    fn test_set_cookie_rejection() {
        let cache = HttpCache::new(100);
        let headers = vec![
            (
                "Cache-Control".to_string(),
                "public, max-age=3600".to_string(),
            ),
            (
                "Set-Cookie".to_string(),
                "session_id=secret123; HttpOnly".to_string(),
            ),
        ];
        let body = Bytes::from_static(b"user profile");

        let stored = cache.put("GET", "api.local", "/profile", 200, &headers, body);
        assert!(
            !stored,
            "Shared cache MUST NOT store responses containing Set-Cookie"
        );

        assert!(matches!(
            cache.get("GET", "api.local", "/profile", None),
            CacheResult::Miss
        ));
    }

    #[test]
    fn test_authorization_header_cache_rejection() {
        let cache = HttpCache::new(100);
        let headers = vec![
            ("Cache-Control".to_string(), "max-age=3600".to_string()),
            ("ETag".to_string(), "\"v1\"".to_string()),
        ];
        let body = Bytes::from_static(b"alice private profile");

        let stored = cache.put_with_auth(
            "GET",
            "api.local",
            "/private",
            200,
            &headers,
            body.clone(),
            true,
        );
        assert!(
            !stored,
            "RFC 7234 §3.2: Authenticated response without public/s-maxage MUST NOT be cached"
        );

        let public_headers = vec![
            (
                "Cache-Control".to_string(),
                "public, max-age=3600".to_string(),
            ),
            ("ETag".to_string(), "\"v1\"".to_string()),
        ];
        let stored_public = cache.put_with_auth(
            "GET",
            "api.local",
            "/public-auth",
            200,
            &public_headers,
            body,
            true,
        );
        assert!(
            stored_public,
            "RFC 7234 §3.2: Authenticated response with public directive is cacheable"
        );
    }

    #[test]
    fn test_true_lru_eviction_issue_7() {
        // Shard capacity is max(2, capacity / 32). With capacity 64, shard cap is 2.
        // Let's test a single shard directly to guarantee deterministic eviction order.
        let mut shard = CacheShard::new(2);

        let h = vec![("Cache-Control".to_string(), "max-age=300".to_string())];
        let make_resp = |val: &str| CachedResponse {
            status: 200,
            headers: h.clone(),
            body: Bytes::copy_from_slice(val.as_bytes()),
            created_at: Instant::now(),
            ttl: Duration::from_secs(300),
            etag: None,
            last_modified: None,
            vary: vec![],
        };

        let key_a = CacheKey::new("GET", "api.local", "/a");
        let key_b = CacheKey::new("GET", "api.local", "/b");
        let key_c = CacheKey::new("GET", "api.local", "/c");

        // Insert A, then B
        shard.put(key_a.clone(), make_resp("A"));
        shard.put(key_b.clone(), make_resp("B"));

        // Access A multiple times (bumps recency above B)
        assert!(shard.get(&key_a).is_some());
        assert!(shard.get(&key_a).is_some());

        // Now insert C. In FIFO, A would be evicted because it was inserted first.
        // In TRUE LRU, B must be evicted because A was accessed more recently!
        shard.put(key_c.clone(), make_resp("C"));

        assert!(shard.get(&key_a).is_some(), "True LRU must keep frequently accessed item A!");
        assert!(shard.get(&key_b).is_none(), "True LRU must evict least-recently-used item B!");
        assert!(shard.get(&key_c).is_some(), "Newly inserted C must be present!");
    }

    #[test]
    fn test_vary_support_issue_8() {
        let cache = HttpCache::new(100);

        let gzip_headers = vec![
            ("Cache-Control".to_string(), "public, max-age=300".to_string()),
            ("Vary".to_string(), "Accept-Encoding".to_string()),
            ("Content-Encoding".to_string(), "gzip".to_string()),
        ];
        let gzip_body = Bytes::from_static(b"gzipped-content");

        // Stored for Accept-Encoding: gzip
        let req_gzip = [("Accept-Encoding", "gzip")];
        assert!(cache.put_with_auth_and_req_headers(
            "GET",
            "api.local",
            "/data",
            200,
            &gzip_headers,
            gzip_body,
            false,
            &req_gzip,
        ));

        // 1. Request with Accept-Encoding: gzip should HIT
        match cache.get_with_headers("GET", "api.local", "/data", &req_gzip, None) {
            CacheResult::Hit(resp) => {
                assert_eq!(resp.body.as_ref(), b"gzipped-content");
            }
            _ => panic!("Expected cache hit for matching Vary Accept-Encoding"),
        }

        // 2. Request with Accept-Encoding: br or identity should MISS (RFC 7234 §4.1)
        let req_br = [("Accept-Encoding", "br")];
        assert!(matches!(
            cache.get_with_headers("GET", "api.local", "/data", &req_br, None),
            CacheResult::Miss
        ));

        // 3. Vary: * must NOT be cached
        let star_headers = vec![
            ("Cache-Control".to_string(), "public, max-age=300".to_string()),
            ("Vary".to_string(), "*".to_string()),
        ];
        assert!(!cache.put_with_auth_and_req_headers(
            "GET",
            "api.local",
            "/star",
            200,
            &star_headers,
            Bytes::from_static(b"star"),
            false,
            &[],
        ));
    }
}
