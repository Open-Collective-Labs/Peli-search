use std::collections::HashSet;

use crate::ranking::statistics::CollectionStats;
use crate::tokenizer::tokenize;

/// Configuration parameters for the BM25 ranking formula.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BM25Config {
    /// Term frequency saturation parameter (default: 1.2).
    pub k1: f32,
    /// Length normalization parameter (default: 0.75).
    pub b: f32,
}

impl Default for BM25Config {
    fn default() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
        }
    }
}

impl BM25Config {
    /// Create a new `BM25Config` with custom parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::bm25::BM25Config;
    ///
    /// let config = BM25Config::new(1.5, 0.5);
    /// assert_eq!(config.k1, 1.5);
    /// assert_eq!(config.b, 0.5);
    /// ```
    pub fn new(k1: f32, b: f32) -> Self {
        Self { k1, b }
    }

    /// Compute the Inverse Document Frequency (IDF) for a term.
    ///
    /// Uses the formula: `ln(1 + (N - n + 0.5) / (n + 0.5))`
    ///
    /// where `N` is the total number of documents and `n` is the
    /// number of documents containing the term.
    ///
    /// Returns `0.0` if `total_docs` is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::bm25::BM25Config;
    ///
    /// let config = BM25Config::default();
    /// let idf = config.idf(10, 100);
    /// assert!(idf > 0.0);
    /// ```
    pub fn idf(&self, doc_freq: u64, total_docs: u64) -> f32 {
        if total_docs == 0 || doc_freq == 0 {
            return 0.0;
        }
        let n = doc_freq as f32;
        let n_total = total_docs as f32;
        (1.0 + (n_total - n + 0.5) / (n + 0.5)).ln()
    }

    /// Compute the BM25 score contribution of a single term in a document.
    ///
    /// Formula: `TF * (k1 + 1) / (TF + k1 * (1 - b + b * doc_len / avg_doc_len))`
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::bm25::BM25Config;
    ///
    /// let config = BM25Config::default();
    /// let score = config.score_term(2.0, 10.0, 5.0);
    /// assert!(score > 0.0);
    /// ```
    pub fn score_term(&self, term_freq: f32, doc_len: f32, avg_doc_len: f32) -> f32 {
        if term_freq <= 0.0 || doc_len <= 0.0 || avg_doc_len <= 0.0 {
            return 0.0;
        }
        let numerator = term_freq * (self.k1 + 1.0);
        let denominator = term_freq + self.k1 * (1.0 - self.b + self.b * doc_len / avg_doc_len);
        numerator / denominator
    }

    /// Compute the BM25 score for a document given a query.
    ///
    /// The query is tokenized, and the score is the sum of
    /// `idf(term) * score_term(term)` for each query term found
    /// in the document.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::bm25::BM25Config;
    /// use pelisearch_core::ranking::statistics::CollectionStats;
    ///
    /// let mut stats = CollectionStats::new();
    /// stats.update_document("doc1", "electric bike review");
    /// stats.update_document("doc2", "electric");
    ///
    /// let config = BM25Config::default();
    /// let score1 = config.score_document("electric bike", "doc1", &stats);
    /// let score2 = config.score_document("electric bike", "doc2", &stats);
    ///
    /// assert!(score1 > score2);
    /// ```
    pub fn score_document(
        &self,
        query: &str,
        doc_id: &str,
        stats: &CollectionStats,
    ) -> f32 {
        let total_docs = stats.total_documents();
        if total_docs == 0 {
            return 0.0;
        }

        let avg_doc_len = stats.average_document_length() as f32;
        if avg_doc_len <= 0.0 {
            return 0.0;
        }

        let doc_len = match stats.get_doc_length(doc_id) {
            Some(len) => len as f32,
            None => return 0.0,
        };

        let query_tokens = tokenize(query);
        let mut seen_terms = HashSet::with_capacity(query_tokens.len());
        let mut total_score = 0.0f32;

        for token in &query_tokens {
            if !seen_terms.insert(token.clone()) {
                continue;
            }

            let term_stats = match stats.get_term_stats(token) {
                Some(s) => s,
                None => continue,
            };

            let tf = stats.get_term_frequency_in_doc(token, doc_id) as f32;
            if tf <= 0.0 {
                continue;
            }

            let idf = self.idf(term_stats.document_frequency, total_docs);
            let term_score = self.score_term(tf, doc_len, avg_doc_len);
            total_score += idf * term_score;
        }

        total_score
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
    fn scores_are_reproducible() {
        let stats = setup_stats();
        let config = BM25Config::default();

        let score1 = config.score_document("electric bike", "doc_a", &stats);
        let score2 = config.score_document("electric bike", "doc_a", &stats);

        assert!((score1 - score2).abs() < 1e-6);
    }

    #[test]
    fn rare_terms_score_higher() {
        let mut stats = CollectionStats::new();
        // "rare" appears in only 1 doc
        stats.update_document("doc1", "rare word");
        // "common" appears in all 4 docs
        stats.update_document("doc1", "common");
        stats.update_document("doc2", "common");
        stats.update_document("doc3", "common");
        stats.update_document("doc4", "common");

        let config = BM25Config::default();
        let rare_score = config.score_document("rare", "doc1", &stats);
        let common_score = config.score_document("common", "doc1", &stats);

        assert!(rare_score > common_score);
    }

    #[test]
    fn common_terms_score_lower() {
        let stats = setup_stats();
        let config = BM25Config::default();

        let doc_a_score = config.score_document("electric bike review", "doc_a", &stats);
        let doc_d_score = config.score_document("electric bike review", "doc_d", &stats);

        // doc_a contains all three terms, doc_d contains none
        assert!(doc_a_score > doc_d_score);
    }

    #[test]
    fn longer_documents_are_normalized() {
        let mut stats = CollectionStats::new();
        // Same term frequency for "hello" but different document lengths
        stats.update_document("doc_short", "hello world");
        stats.update_document("doc_long", "hello world and some extra words here");

        let config = BM25Config::default();
        let short_score = config.score_document("hello", "doc_short", &stats);
        let long_score = config.score_document("hello", "doc_long", &stats);

        // Shorter doc should score higher for the same term match
        assert!(short_score > long_score);
    }

    #[test]
    fn idf_increases_with_rarity() {
        let config = BM25Config::default();
        let rare = config.idf(1, 100);
        let common = config.idf(90, 100);

        assert!(rare > common);
    }

    #[test]
    fn idf_zero_when_no_docs() {
        let config = BM25Config::default();
        assert!((config.idf(5, 0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn idf_zero_when_no_term_occurrences() {
        let config = BM25Config::default();
        assert!((config.idf(0, 100) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn score_term_zero_when_no_term_freq() {
        let config = BM25Config::default();
        let score = config.score_term(0.0, 10.0, 5.0);
        assert!((score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn score_term_zero_when_no_doc_len() {
        let config = BM25Config::default();
        let score = config.score_term(2.0, 0.0, 5.0);
        assert!((score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn empty_query_returns_zero() {
        let stats = setup_stats();
        let config = BM25Config::default();
        let score = config.score_document("", "doc_a", &stats);
        assert!((score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn nonexistent_document_returns_zero() {
        let stats = setup_stats();
        let config = BM25Config::default();
        let score = config.score_document("electric", "nonexistent", &stats);
        assert!((score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn nonexistent_term_returns_zero() {
        let stats = setup_stats();
        let config = BM25Config::default();
        let score = config.score_document("zzzzz", "doc_a", &stats);
        assert!((score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn default_config_values() {
        let config = BM25Config::default();
        assert!((config.k1 - 1.2).abs() < 1e-6);
        assert!((config.b - 0.75).abs() < 1e-6);
    }

    #[test]
    fn custom_config_values() {
        let config = BM25Config::new(1.5, 0.3);
        assert!((config.k1 - 1.5).abs() < 1e-6);
        assert!((config.b - 0.3).abs() < 1e-6);
    }
}
