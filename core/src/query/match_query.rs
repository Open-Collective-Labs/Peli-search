use serde::{Deserialize, Serialize};

/// A full-text match query.
///
/// The `value` is analyzed (tokenized) before matching against the index.
/// This is the primary query type for human-readable text fields.
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
    /// The field to search in.
    pub field: String,
    /// The text value to match (analyzed).
    pub value: String,
}

impl MatchQuery {
    /// Create a new `MatchQuery`.
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
