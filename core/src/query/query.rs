use serde::{Deserialize, Serialize};

use crate::query::match_query::MatchQuery;
use crate::query::range_query::RangeQuery;
use crate::query::term_query::TermQuery;

/// A structured search query.
///
/// Each variant represents a different kind of query constraint that can be
/// combined with others through higher-level constructs.
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::Query;
/// use pelisearch_core::query::MatchQuery;
///
/// let q = Query::Match(MatchQuery::new("title", "electric bike"));
/// assert_eq!(format!("{q:?}"), "Match(MatchQuery { field: \"title\", value: \"electric bike\" })");
/// ```
///
/// # Serialization
///
/// The enum is tagged with `"type"`:
///
/// ```json
/// {"type": "Match", "field": "title", "value": "electric bike"}
/// {"type": "Term", "field": "category", "value": "electronics"}
/// {"type": "Range", "field": "price", "lte": 1000.0}
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Query {
    /// Full-text match query (analyzed).
    Match(MatchQuery),
    /// Exact term query (not analyzed).
    Term(TermQuery),
    /// Numeric range filter.
    Range(RangeQuery),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_match_query() {
        let q = Query::Match(MatchQuery::new("title", "hello world"));
        assert!(matches!(q, Query::Match(_)));
    }

    #[test]
    fn create_term_query() {
        let q = Query::Term(TermQuery::new("category", "electronics"));
        assert!(matches!(q, Query::Term(_)));
    }

    #[test]
    fn create_range_query() {
        let q = Query::Range(RangeQuery::new("price").with_lte(1000.0));
        assert!(matches!(q, Query::Range(_)));
    }

    #[test]
    fn query_serde_roundtrip_match() {
        let q = Query::Match(MatchQuery::new("title", "electric bike"));
        let json = serde_json::to_string(&q).unwrap();
        let deserialized: Query = serde_json::from_str(&json).unwrap();
        assert_eq!(q, deserialized);
    }

    #[test]
    fn query_serde_roundtrip_term() {
        let q = Query::Term(TermQuery::new("status", "active"));
        let json = serde_json::to_string(&q).unwrap();
        let deserialized: Query = serde_json::from_str(&json).unwrap();
        assert_eq!(q, deserialized);
    }

    #[test]
    fn query_serde_roundtrip_range() {
        let q = Query::Range(RangeQuery::new("price").with_gte(10.0).with_lte(100.0));
        let json = serde_json::to_string(&q).unwrap();
        let deserialized: Query = serde_json::from_str(&json).unwrap();
        assert_eq!(q, deserialized);
    }

    #[test]
    fn query_debug_output() {
        let q = Query::Match(MatchQuery::new("title", "test"));
        let debug = format!("{q:?}");
        assert!(debug.contains("title"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn query_variants_are_distinct() {
        let m = Query::Match(MatchQuery::new("a", "b"));
        let t = Query::Term(TermQuery::new("a", "b"));
        assert_ne!(m, t);
    }

    #[test]
    fn query_clone() {
        let a = Query::Match(MatchQuery::new("f", "v"));
        let b = a.clone();
        assert_eq!(a, b);
    }
}
