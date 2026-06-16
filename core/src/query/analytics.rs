use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Collects search analytics: popular queries, latency, volume, cache stats.
///
/// Thread-safe: uses interior mutability for the query frequency maps.
pub struct SearchAnalytics {
    top_queries: Mutex<HashMap<String, u64>>,
    zero_result_queries: Mutex<HashMap<String, u64>>,
    total_queries: AtomicU64,
    total_latency_ns: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    max_top_queries: usize,
}

impl SearchAnalytics {
    /// Create a new analytics collector.
    pub fn new() -> Self {
        Self {
            top_queries: Mutex::new(HashMap::new()),
            zero_result_queries: Mutex::new(HashMap::new()),
            total_queries: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            max_top_queries: 100,
        }
    }

    /// Record a search query execution.
    pub fn record_query(
        &self,
        query: &str,
        latency_ns: u64,
        num_results: usize,
    ) {
        self.total_queries.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ns.fetch_add(latency_ns, Ordering::Relaxed);

        // Track top queries (bounded)
        let mut top = self.top_queries.lock().unwrap();
        *top.entry(query.to_string()).or_insert(0) += 1;
        if top.len() > self.max_top_queries {
            shrink_to_half(&mut top);
        }

        // Track zero-result queries
        if num_results == 0 {
            let mut zero = self.zero_result_queries.lock().unwrap();
            *zero.entry(query.to_string()).or_insert(0) += 1;
            if zero.len() > self.max_top_queries {
                shrink_to_half(&mut zero);
            }
        }
    }

    /// Record cache result (hit or miss).
    pub fn record_cache(&self, hit: bool) {
        if hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Total number of queries executed.
    pub fn total_queries(&self) -> u64 {
        self.total_queries.load(Ordering::Relaxed)
    }

    /// Average latency in nanoseconds.
    pub fn average_latency_ns(&self) -> f64 {
        let total = self.total_queries();
        if total == 0 {
            return 0.0;
        }
        self.total_latency_ns.load(Ordering::Relaxed) as f64 / total as f64
    }

    /// Total cache hits.
    pub fn cache_hits(&self) -> u64 {
        self.cache_hits.load(Ordering::Relaxed)
    }

    /// Total cache misses.
    pub fn cache_misses(&self) -> u64 {
        self.cache_misses.load(Ordering::Relaxed)
    }

    /// Cache hit rate (0.0 - 1.0).
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits();
        let misses = self.cache_misses();
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Top N queries by frequency.
    pub fn top_queries(&self, n: usize) -> Vec<(String, u64)> {
        let top = self.top_queries.lock().unwrap();
        let mut sorted: Vec<(String, u64)> = top.iter().map(|(k, v)| (k.clone(), *v)).collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }

    /// Top N zero-result queries by frequency.
    pub fn zero_result_queries(&self, n: usize) -> Vec<(String, u64)> {
        let zero = self.zero_result_queries.lock().unwrap();
        let mut sorted: Vec<(String, u64)> = zero.iter().map(|(k, v)| (k.clone(), *v)).collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }

    /// Reset all analytics counters and data.
    pub fn reset(&self) {
        self.total_queries.store(0, Ordering::Relaxed);
        self.total_latency_ns.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.top_queries.lock().unwrap().clear();
        self.zero_result_queries.lock().unwrap().clear();
    }
}

fn shrink_to_half(map: &mut HashMap<String, u64>) {
    let target = map.len() / 2;
    let mut sorted: Vec<(String, u64)> = map.drain().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.truncate(target);
    for (k, v) in sorted {
        map.insert(k, v);
    }
}

impl Default for SearchAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_query_increment() {
        let analytics = SearchAnalytics::new();
        analytics.record_query("rust", 1_000_000, 5);
        assert_eq!(analytics.total_queries(), 1);
    }

    #[test]
    fn average_latency() {
        let analytics = SearchAnalytics::new();
        analytics.record_query("a", 100, 1);
        analytics.record_query("b", 300, 1);
        let avg = analytics.average_latency_ns();
        assert!((avg - 200.0).abs() < 1e-6);
    }

    #[test]
    fn zero_latency_when_no_queries() {
        let analytics = SearchAnalytics::new();
        assert!((analytics.average_latency_ns() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn tracks_zero_result_queries() {
        let analytics = SearchAnalytics::new();
        analytics.record_query("nonexistent", 100, 0);
        analytics.record_query("found", 100, 5);
        let zero = analytics.zero_result_queries(10);
        assert_eq!(zero.len(), 1);
        assert_eq!(zero[0].0, "nonexistent");
    }

    #[test]
    fn top_queries_ordered_by_frequency() {
        let analytics = SearchAnalytics::new();
        analytics.record_query("b", 100, 1);
        analytics.record_query("a", 100, 1);
        analytics.record_query("a", 100, 1);
        let top = analytics.top_queries(10);
        assert_eq!(top[0].0, "a");
        assert_eq!(top[0].1, 2);
        assert_eq!(top[1].0, "b");
        assert_eq!(top[1].1, 1);
    }

    #[test]
    fn top_queries_respects_limit() {
        let analytics = SearchAnalytics::new();
        analytics.record_query("a", 100, 1);
        analytics.record_query("b", 100, 1);
        analytics.record_query("c", 100, 1);
        let top = analytics.top_queries(2);
        assert_eq!(top.len(), 2);
    }

    #[test]
    fn cache_hit_rate() {
        let analytics = SearchAnalytics::new();
        analytics.record_cache(true);
        analytics.record_cache(true);
        analytics.record_cache(false);
        assert!((analytics.cache_hit_rate() - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn cache_hit_rate_zero_when_no_requests() {
        let analytics = SearchAnalytics::new();
        assert!((analytics.cache_hit_rate() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn reset_clears_all() {
        let analytics = SearchAnalytics::new();
        analytics.record_query("test", 100, 5);
        analytics.record_cache(true);
        analytics.reset();
        assert_eq!(analytics.total_queries(), 0);
        assert_eq!(analytics.cache_hits(), 0);
        assert!(analytics.top_queries(10).is_empty());
        assert!(analytics.zero_result_queries(10).is_empty());
    }
}
