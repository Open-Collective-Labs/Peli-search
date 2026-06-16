use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A full-text match query.
///
/// The `value` is analyzed (tokenized) before matching against the index.
/// This is the primary query type for human-readable text fields.
///
/// Supports optional per-term boosting and field-level search.
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::MatchQuery;
///
/// let q = MatchQuery::new("title", "electric bike");
/// assert_eq!(q.field, "title");
/// assert_eq!(q.value, "electric bike");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchQuery {
    /// The field to search in. Empty string means all indexed text.
    pub field: String,
    /// The text value to match (analyzed).
    pub value: String,
    /// Per-term boost factors. If empty, all terms have boost 1.0.
    /// Maps token -> boost multiplier.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub term_boosts: HashMap<String, f32>,
    /// Overall query boost multiplier (default: 1.0).
    #[serde(default = "one_f32")]
    pub boost: f32,
}

fn one_f32() -> f32 {
    1.0
}

impl MatchQuery {
    /// Create a new `MatchQuery`.
    pub fn new(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
            term_boosts: HashMap::new(),
            boost: 1.0,
        }
    }

    /// Set the overall query boost.
    pub fn with_boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }

    /// Set the overall query boost (builder-style, deprecated name for with_boost).
    pub fn set_boost(&mut self, boost: f32) {
        self.boost = boost;
    }

    /// Boost a specific term in the query.
    /// Terms not explicitly boosted default to 1.0.
    pub fn boost_term(mut self, term: impl Into<String>, boost: f32) -> Self {
        self.term_boosts.insert(term.into(), boost);
        self
    }

    /// Get the effective boost for a term (overall * per-term).
    pub fn boost(&self, term: &str) -> f32 {
        self.boost * self.term_boosts.get(term).copied().unwrap_or(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_match_query() {
        let q = MatchQuery::new("title", "hello world");
        assert_eq!(q.field, "title");
        assert_eq!(q.value, "hello world");
    }

    #[test]
    fn match_query_empty_field() {
        let q = MatchQuery::new("", "value");
        assert_eq!(q.field, "");
    }

    #[test]
    fn match_query_empty_value() {
        let q = MatchQuery::new("field", "");
        assert_eq!(q.value, "");
    }

    #[test]
    fn match_query_serde_roundtrip() {
        let q = MatchQuery::new("body", "lorem ipsum");
        let json = serde_json::to_string(&q).unwrap();
        let deserialized: MatchQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q, deserialized);
    }

    #[test]
    fn match_query_debug_output() {
        let q = MatchQuery::new("title", "test");
        let debug = format!("{q:?}");
        assert!(debug.contains("title"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn match_query_clone() {
        let a = MatchQuery::new("x", "y");
        let b = a.clone();
        assert_eq!(a, b);
    }
}
