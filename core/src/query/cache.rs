use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::types::SearchHit;

const DEFAULT_CACHE_CAPACITY: usize = 1000;

/// A fixed-capacity LRU query result cache.
///
/// Thread-safe: uses interior mutability with a `Mutex`.
pub struct QueryCache {
    inner: Mutex<LruInner>,
    hits: AtomicU64,
    misses: AtomicU64,
}

struct LruInner {
    capacity: usize,
    entries: HashMap<u64, CacheEntry>,
    order: Vec<u64>,
}

struct CacheEntry {
    results: Vec<SearchHit>,
}

impl QueryCache {
    /// Create a new cache with the default capacity (1000 entries).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CACHE_CAPACITY)
    }

    /// Create a new cache with a specific capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(LruInner {
                capacity,
                entries: HashMap::new(),
                order: Vec::with_capacity(capacity),
            }),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Compute a cache key from a query value and pagination parameters.
    pub fn cache_key(query: &str, from: usize, size: usize) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        query.hash(&mut hasher);
        from.hash(&mut hasher);
        size.hash(&mut hasher);
        hasher.finish()
    }

    /// Try to get cached results for a key.
    pub fn get(&self, key: u64) -> Option<Vec<SearchHit>> {
        let mut inner = self.inner.lock().unwrap();
        if inner.entries.contains_key(&key) {
            // Move to front (most recently used)
            if let Some(pos) = inner.order.iter().position(|k| *k == key) {
                inner.order.remove(pos);
                inner.order.push(key);
            }
            self.hits.fetch_add(1, Ordering::Relaxed);
            inner.entries.get(&key).map(|e| e.results.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Insert results into the cache.
    pub fn insert(&self, key: u64, results: Vec<SearchHit>) {
        let mut inner = self.inner.lock().unwrap();

        // Evict if at capacity
        if inner.entries.len() >= inner.capacity && !inner.entries.contains_key(&key) {
            if let Some(lru_key) = inner.order.first().copied() {
                inner.entries.remove(&lru_key);
                inner.order.remove(0);
            }
        }

        inner.entries.insert(
            key,
            CacheEntry {
                results: results.clone(),
            },
        );
        // Remove from order if exists, then push to end (most recent)
        if let Some(pos) = inner.order.iter().position(|k| *k == key) {
            inner.order.remove(pos);
        }
        inner.order.push(key);
    }

    /// Number of cache hits.
    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Number of cache misses.
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Current number of entries in the cache.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().entries.len()
    }

    /// Is the cache empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.entries.clear();
        inner.order.clear();
    }

    /// Hit rate (0.0 - 1.0).
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits() as f64;
        let misses = self.misses() as f64;
        let total = hits + misses;
        if total == 0.0 {
            0.0
        } else {
            hits / total
        }
    }
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hits(n: usize) -> Vec<SearchHit> {
        (0..n)
            .map(|i| SearchHit::new("test", format!("doc{i}"), 1.0))
            .collect()
    }

    #[test]
    fn cache_hit_returns_results() {
        let cache = QueryCache::new();
        let key = 42;
        let results = make_hits(3);
        cache.insert(key, results.clone());
        assert_eq!(cache.get(key), Some(results));
    }

    #[test]
    fn cache_miss_returns_none() {
        let cache = QueryCache::new();
        assert_eq!(cache.get(999), None);
    }

    #[test]
    fn cache_tracks_hits_and_misses() {
        let cache = QueryCache::new();
        cache.insert(1, make_hits(1));
        let _ = cache.get(1);
        let _ = cache.get(2);
        assert_eq!(cache.hits(), 1);
        assert_eq!(cache.misses(), 1);
    }

    #[test]
    fn cache_hit_rate() {
        let cache = QueryCache::new();
        cache.insert(1, make_hits(1));
        let _ = cache.get(1);
        let _ = cache.get(2);
        let rate = cache.hit_rate();
        assert!((rate - 0.5).abs() < 1e-6);
    }

    #[test]
    fn eviction_removes_oldest() {
        let cache = QueryCache::with_capacity(2);
        cache.insert(1, make_hits(1));
        cache.insert(2, make_hits(1));
        cache.insert(3, make_hits(1)); // should evict key 1
        assert_eq!(cache.get(1), None);
        assert_eq!(cache.get(2).is_some(), true);
        assert_eq!(cache.get(3).is_some(), true);
    }

    #[test]
    fn lru_refresh_prevents_eviction() {
        let cache = QueryCache::with_capacity(2);
        cache.insert(1, make_hits(1));
        cache.insert(2, make_hits(1));
        // Access key 1 — makes it recently used
        let _ = cache.get(1);
        cache.insert(3, make_hits(1)); // should evict key 2
        assert_eq!(cache.get(1).is_some(), true);
        assert_eq!(cache.get(2), None);
        assert_eq!(cache.get(3).is_some(), true);
    }

    #[test]
    fn clear_empties_cache() {
        let cache = QueryCache::new();
        cache.insert(1, make_hits(1));
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.get(1), None);
    }

    #[test]
    fn cache_key_different_queries_different_keys() {
        let k1 = QueryCache::cache_key("rust", 0, 10);
        let k2 = QueryCache::cache_key("java", 0, 10);
        assert_ne!(k1, k2);
    }

    #[test]
    fn cache_key_different_pagination_different_keys() {
        let k1 = QueryCache::cache_key("rust", 0, 10);
        let k2 = QueryCache::cache_key("rust", 10, 10);
        assert_ne!(k1, k2);
    }

    #[test]
    fn cache_key_same_query_same_key() {
        let k1 = QueryCache::cache_key("rust", 0, 10);
        let k2 = QueryCache::cache_key("rust", 0, 10);
        assert_eq!(k1, k2);
    }
}
