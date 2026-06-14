/// A single search result entry.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// The ID of the matching document.
    pub document_id: String,
    /// The relevance score (higher is more relevant).
    pub score: f64,
}

impl SearchResult {
    /// Create a new `SearchResult`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::types::SearchResult;
    ///
    /// let r = SearchResult::new("doc1", 0.85);
    /// assert_eq!(r.document_id, "doc1");
    /// assert!((r.score - 0.85).abs() < 1e-10);
    /// ```
    pub fn new(document_id: impl Into<String>, score: f64) -> Self {
        Self {
            document_id: document_id.into(),
            score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_search_result() {
        let r = SearchResult::new("abc", 1.0);
        assert_eq!(r.document_id, "abc");
        assert!((r.score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn search_result_zero_score() {
        let r = SearchResult::new("doc1", 0.0);
        assert!((r.score - 0.0).abs() < 1e-10);
    }

    #[test]
    fn search_result_negative_score() {
        let r = SearchResult::new("doc1", -1.5);
        assert!((r.score - (-1.5)).abs() < 1e-10);
    }

    #[test]
    fn search_result_equality() {
        let a = SearchResult::new("doc1", 0.5);
        let b = SearchResult::new("doc1", 0.5);
        assert_eq!(a, b);
    }
}
