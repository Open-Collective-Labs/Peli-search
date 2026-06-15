use serde::{Deserialize, Serialize};

use crate::query::Query;

/// A structured search request combining a main query with optional filters.
///
/// The `query` field holds the primary search intent (e.g. a `Match` against a
/// text field). The `filters` slice holds additional constraints (e.g. `Term`
/// or `Range` queries) that narrow the candidate set before ranking.
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::{Query, MatchQuery, TermQuery, RangeQuery, SearchRequest};
///
/// let request = SearchRequest {
///     query: Query::Match(MatchQuery::new("title", "bike")),
///     filters: vec![
///         Query::Term(TermQuery::new("category", "electronics")),
///         Query::Range(RangeQuery::new("price").with_lte(1000.0)),
///     ],
/// };
///
/// assert!(matches!(request.query, Query::Match(_)));
/// assert_eq!(request.filters.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchRequest {
    /// The primary search query.
    pub query: Query,
    /// Additional filter constraints applied before ranking.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<Query>,
}

#[cfg(test)]
mod tests {
    use crate::query::{MatchQuery, Query, RangeQuery, SearchRequest, TermQuery};

    #[test]
    fn create_request_with_query_only() {
        let req = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "hello")),
            filters: vec![],
        };
        assert!(req.filters.is_empty());
    }

    #[test]
    fn create_request_with_match_and_filters() {
        let req = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![
                Query::Term(TermQuery::new("category", "electronics")),
                Query::Range(RangeQuery::new("price").with_lte(1000.0)),
            ],
        };
        assert_eq!(req.filters.len(), 2);
    }

    #[test]
    fn serde_roundtrip() {
        let req = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![
                Query::Term(TermQuery::new("category", "electronics")),
            ],
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: SearchRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);
    }

    #[test]
    fn serde_omits_empty_filters() {
        let req = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "test")),
            filters: vec![],
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("filters"));
    }

    #[test]
    fn debug_output() {
        let req = SearchRequest {
            query: Query::Match(MatchQuery::new("f", "v")),
            filters: vec![],
        };
        let debug = format!("{req:?}");
        assert!(debug.contains("f"));
        assert!(debug.contains("v"));
    }

    #[test]
    fn clone() {
        let a = SearchRequest {
            query: Query::Match(MatchQuery::new("t", "v")),
            filters: vec![],
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
