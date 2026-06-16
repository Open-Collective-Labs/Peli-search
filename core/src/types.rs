use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A single search result entry.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// The ID of the matching document.
    pub document_id: String,
    /// The relevance score (higher is more relevant).
    pub score: f64,
}

impl SearchResult {
    pub fn new(document_id: impl Into<String>, score: f64) -> Self {
        Self {
            document_id: document_id.into(),
            score,
        }
    }
}

/// A search hit that identifies the source index alongside the result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    /// The name of the index that matched.
    pub index: String,
    /// The ID of the matching document.
    pub document_id: String,
    /// The relevance score (higher is more relevant).
    pub score: f64,
    /// Optional per-field highlighted snippets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlighted: Option<HashMap<String, String>>,
}

impl SearchHit {
    pub fn new(index: impl Into<String>, document_id: impl Into<String>, score: f64) -> Self {
        Self {
            index: index.into(),
            document_id: document_id.into(),
            score,
            highlighted: None,
        }
    }

    /// Attach highlighted fields to this hit.
    pub fn with_highlights(mut self, highlights: HashMap<String, String>) -> Self {
        self.highlighted = Some(highlights);
        self
    }
}

/// Aggregation results keyed by aggregation name (field name).
pub type AggregationResults = HashMap<String, serde_json::Value>;

/// A search response containing hits and aggregation results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Ranked search hits.
    pub hits: Vec<SearchHit>,
    /// Aggregation results keyed by aggregation name.
    #[serde(default)]
    pub aggregations: AggregationResults,
    /// Total number of matching hits before pagination.
    pub total: usize,
}

impl SearchResponse {
    pub fn new(hits: Vec<SearchHit>, aggregations: AggregationResults, total: usize) -> Self {
        Self { hits, aggregations, total }
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

    #[test]
    fn create_search_hit() {
        let h = SearchHit::new("products", "doc1", 7.42);
        assert_eq!(h.index, "products");
        assert_eq!(h.document_id, "doc1");
        assert!((h.score - 7.42).abs() < 1e-10);
    }

    #[test]
    fn search_hit_with_highlights() {
        let mut highlights = HashMap::new();
        highlights.insert("title".to_string(), "Learning <em>Rust</em>".to_string());
        let h = SearchHit::new("docs", "doc1", 1.0).with_highlights(highlights.clone());
        assert_eq!(h.highlighted, Some(highlights));
    }

    #[test]
    fn search_hit_equality() {
        let a = SearchHit::new("idx", "doc1", 1.0);
        let b = SearchHit::new("idx", "doc1", 1.0);
        assert_eq!(a, b);
    }

    #[test]
    fn search_hit_different_index_not_equal() {
        let a = SearchHit::new("products", "doc1", 1.0);
        let b = SearchHit::new("users", "doc1", 1.0);
        assert_ne!(a, b);
    }

    #[test]
    fn search_response_creation() {
        let hits = vec![SearchHit::new("idx", "doc1", 1.0)];
        let aggs = AggregationResults::new();
        let response = SearchResponse::new(hits.clone(), aggs, 1);
        assert_eq!(response.hits, hits);
        assert!(response.aggregations.is_empty());
        assert_eq!(response.total, 1);
    }

    #[test]
    fn search_response_serde() {
        let response = SearchResponse {
            hits: vec![SearchHit::new("idx", "doc1", 0.5)],
            aggregations: AggregationResults::new(),
            total: 1,
        };
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: SearchResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response, deserialized);
    }
}
