use crate::ranking::bm25::BM25Config;
use crate::ranking::statistics::CollectionStats;
use crate::tokenizer::tokenize;
use crate::types::SearchResult;

/// A ranking engine that scores documents using BM25.
///
/// Coordinates the statistics engine and BM25 formula to produce
/// ranked search results.
///
/// # Examples
///
/// ```
/// use pelisearch_core::ranking::scorer::Scorer;
/// use pelisearch_core::ranking::statistics::CollectionStats;
///
/// let mut stats = CollectionStats::new();
/// stats.update_document("doc1", "electric bike review");
/// stats.update_document("doc2", "electric");
///
/// let scorer = Scorer::new(&stats);
/// let results = scorer.score_documents("electric bike", &["doc1", "doc2"]);
///
/// assert_eq!(results.len(), 2);
/// assert!(results[0].score > results[1].score);
/// ```
pub struct Scorer<'a> {
    config: BM25Config,
    stats: &'a CollectionStats,
}

impl<'a> Scorer<'a> {
    /// Create a new `Scorer` with default BM25 parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::scorer::Scorer;
    /// use pelisearch_core::ranking::statistics::CollectionStats;
    ///
    /// let stats = CollectionStats::new();
    /// let scorer = Scorer::new(&stats);
    /// ```
    pub fn new(stats: &'a CollectionStats) -> Self {
        Self {
            config: BM25Config::default(),
            stats,
        }
    }

    /// Create a new `Scorer` with custom BM25 parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::scorer::Scorer;
    /// use pelisearch_core::ranking::statistics::CollectionStats;
    ///
    /// let stats = CollectionStats::new();
    /// let scorer = Scorer::with_config(&stats, 1.5, 0.3);
    /// ```
    pub fn with_config(stats: &'a CollectionStats, k1: f32, b: f32) -> Self {
        Self {
            config: BM25Config::new(k1, b),
            stats,
        }
    }

    /// Score a single document against the query using BM25.
    ///
    /// Returns the BM25 score as an `f64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::scorer::Scorer;
    /// use pelisearch_core::ranking::statistics::CollectionStats;
    ///
    /// let mut stats = CollectionStats::new();
    /// stats.update_document("doc1", "electric bike review");
    /// let scorer = Scorer::new(&stats);
    ///
    /// let score = scorer.score_document("electric", "doc1");
    /// assert!(score > 0.0);
    /// ```
    pub fn score_document(&self, query: &str, doc_id: &str) -> f64 {
        self.config.score_document(query, doc_id, self.stats) as f64
    }

    /// Score multiple documents against the query and return ranked results.
    ///
    /// Documents that have no matching terms receive a score of 0.0
    /// and are excluded from the results.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::scorer::Scorer;
    /// use pelisearch_core::ranking::statistics::CollectionStats;
    ///
    /// let mut stats = CollectionStats::new();
    /// stats.update_document("doc1", "electric bike review");
    /// stats.update_document("doc2", "electric");
    /// stats.update_document("doc3", "walking");
    ///
    /// let scorer = Scorer::new(&stats);
    /// let results = scorer.score_documents("electric bike", &["doc1", "doc2", "doc3"]);
    ///
    /// // doc3 has no matching terms, so only 2 results
    /// assert_eq!(results.len(), 2);
    /// ```
    pub fn score_documents(&self, query: &str, doc_ids: &[&str]) -> Vec<SearchResult> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() || doc_ids.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<SearchResult> = doc_ids
            .iter()
            .filter_map(|doc_id| {
                let score = self.config.score_document(query, doc_id, self.stats);
                if score <= 0.0 {
                    return None;
                }
                Some(SearchResult::new(doc_id.to_string(), score as f64))
            })
            .collect();

        self.sort_results(&mut results);
        results
    }

    /// Sort search results by score descending with stable ordering
    /// (doc_id tiebreaker).
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::scorer::Scorer;
    /// use pelisearch_core::ranking::statistics::CollectionStats;
    /// use pelisearch_core::types::SearchResult;
    ///
    /// let stats = CollectionStats::new();
    /// let scorer = Scorer::new(&stats);
    ///
    /// let mut results = vec![
    ///     SearchResult::new("b", 1.0),
    ///     SearchResult::new("a", 2.0),
    ///     SearchResult::new("c", 1.0),
    /// ];
    /// scorer.sort_results(&mut results);
    ///
    /// assert_eq!(results[0].document_id, "a");
    /// assert_eq!(results[1].document_id, "b");
    /// assert_eq!(results[2].document_id, "c");
    /// ```
    pub fn sort_results(&self, results: &mut [SearchResult]) {
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.document_id.cmp(&b.document_id))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_stats() -> CollectionStats {
        let mut stats = CollectionStats::new();
        stats.update_document("doc_a", "electric bike review");
        stats.update_document("doc_b", "electric");
        stats.update_document("doc_c", "bike for commuting");
        stats.update_document("doc_d", "walking in the park");
        stats
    }

    #[test]
    fn results_sorted_descending() {
        let stats = setup_stats();
        let scorer = Scorer::new(&stats);
        let results = scorer.score_documents("electric bike", &["doc_a", "doc_b", "doc_c", "doc_d"]);

        assert!(results.windows(2).all(|w| w[0].score >= w[1].score));
    }

    #[test]
    fn stable_ordering() {
        let mut stats = CollectionStats::new();
        // All docs contain "hello" once, same length
        stats.update_document("doc_b", "hello");
        stats.update_document("doc_a", "hello");
        stats.update_document("doc_c", "hello");

        let scorer = Scorer::new(&stats);
        let results = scorer.score_documents("hello", &["doc_a", "doc_b", "doc_c"]);

        // All have same score, should be sorted by doc_id ascending
        assert_eq!(results[0].document_id, "doc_a");
        assert_eq!(results[1].document_id, "doc_b");
        assert_eq!(results[2].document_id, "doc_c");
    }

    #[test]
    fn deterministic_output() {
        let stats = setup_stats();
        let scorer = Scorer::new(&stats);

        let results1 = scorer.score_documents("electric bike", &["doc_a", "doc_b", "doc_c"]);
        let results2 = scorer.score_documents("electric bike", &["doc_a", "doc_b", "doc_c"]);

        assert_eq!(results1, results2);
    }

    #[test]
    fn score_document_returns_f64() {
        let stats = setup_stats();
        let scorer = Scorer::new(&stats);

        let score = scorer.score_document("electric", "doc_a");
        assert!(score > 0.0);
    }

    #[test]
    fn empty_query_no_results() {
        let stats = setup_stats();
        let scorer = Scorer::new(&stats);
        let results = scorer.score_documents("", &["doc_a"]);
        assert!(results.is_empty());
    }

    #[test]
    fn empty_doc_ids_no_results() {
        let stats = setup_stats();
        let scorer = Scorer::new(&stats);
        let results = scorer.score_documents("electric", &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn no_matching_docs_excluded() {
        let stats = setup_stats();
        let scorer = Scorer::new(&stats);
        let results = scorer.score_documents("zzzzz", &["doc_a", "doc_b"]);
        assert!(results.is_empty());
    }

    #[test]
    fn custom_config_scorer() {
        let stats = setup_stats();
        let scorer = Scorer::with_config(&stats, 1.5, 0.3);

        let results = scorer.score_documents("electric bike", &["doc_a", "doc_b"]);
        assert_eq!(results.len(), 2);
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn sort_results_tiebreaker() {
        let stats = CollectionStats::new();
        let scorer = Scorer::new(&stats);

        let mut results = vec![
            SearchResult::new("z", 2.0),
            SearchResult::new("a", 2.0),
            SearchResult::new("m", 1.0),
        ];
        scorer.sort_results(&mut results);

        assert_eq!(results[0].document_id, "a");
        assert_eq!(results[1].document_id, "z");
        assert_eq!(results[2].document_id, "m");
    }
}
