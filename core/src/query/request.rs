use serde::{Deserialize, Serialize};

use crate::aggregation::Aggregation;
use crate::query::Query;
use crate::sort::SortField;

/// A structured search request combining a main query with optional filters,
/// sort specifications, and aggregations.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::{Aggregation, TermsAggregation};
/// use pelisearch_core::query::{Query, MatchQuery, RangeQuery, SearchRequest};
/// use pelisearch_core::sort::SortField;
///
/// let request = SearchRequest {
///     query: Query::Match(MatchQuery::new("title", "bike")),
///     filters: vec![
///         Query::Range(RangeQuery::new("price").with_lte(1000.0)),
///     ],
///     sort: vec![SortField::asc("price")],
///     aggregations: vec![
///         Aggregation::Terms(TermsAggregation::new("category")),
///     ],
/// };
///
/// assert!(matches!(request.query, Query::Match(_)));
/// assert_eq!(request.filters.len(), 1);
/// assert_eq!(request.sort.len(), 1);
/// assert_eq!(request.aggregations.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchRequest {
    /// The primary search query.
    pub query: Query,
    /// Additional filter constraints applied before ranking.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<Query>,
    /// Sort specifications for ordering results.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sort: Vec<SortField>,
    /// Aggregations for computing summary metrics.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aggregations: Vec<Aggregation>,
}

#[cfg(test)]
mod tests {
    use crate::query::{MatchQuery, Query, RangeQuery, SearchRequest, TermQuery};

    #[test]
    fn create_request_with_query_only() {
        let req = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "hello")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
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
            sort: vec![],
            aggregations: vec![],
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
            sort: vec![],
            aggregations: vec![],
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
            sort: vec![],
            aggregations: vec![],
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("filters"));
    }

    #[test]
    fn debug_output() {
        let req = SearchRequest {
            query: Query::Match(MatchQuery::new("f", "v")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
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
            sort: vec![],
            aggregations: vec![],
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
