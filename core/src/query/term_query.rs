use serde::{Deserialize, Serialize};

/// An exact-term query.
///
/// The `value` is used as-is (not analyzed) to match against keyword fields.
/// Unlike `MatchQuery`, no tokenization or stemming is applied.
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::TermQuery;
///
/// let q = TermQuery::new("category", "electronics");
/// assert_eq!(q.field, "category");
/// assert_eq!(q.value, "electronics");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TermQuery {
    /// The field to search in.
    pub field: String,
    /// The exact value to match (not analyzed).
    pub value: String,
}

impl TermQuery {
    /// Create a new `TermQuery`.
    pub fn new(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_term_query() {
        let q = TermQuery::new("status", "active");
        assert_eq!(q.field, "status");
        assert_eq!(q.value, "active");
    }

    #[test]
    fn term_query_empty_field() {
        let q = TermQuery::new("", "value");
        assert_eq!(q.field, "");
    }

    #[test]
    fn term_query_empty_value() {
        let q = TermQuery::new("field", "");
        assert_eq!(q.value, "");
    }

    #[test]
    fn term_query_serde_roundtrip() {
        let q = TermQuery::new("category", "electronics");
        let json = serde_json::to_string(&q).unwrap();
        let deserialized: TermQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q, deserialized);
    }

    #[test]
    fn term_query_debug_output() {
        let q = TermQuery::new("color", "red");
        let debug = format!("{q:?}");
        assert!(debug.contains("color"));
        assert!(debug.contains("red"));
    }

    #[test]
    fn term_query_clone() {
        let a = TermQuery::new("x", "y");
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn term_query_partial_eq() {
        let a = TermQuery::new("a", "b");
        let b = TermQuery::new("a", "b");
        let c = TermQuery::new("a", "c");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
