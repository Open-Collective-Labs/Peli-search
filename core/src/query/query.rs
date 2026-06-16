use serde::{Deserialize, Serialize};

use crate::query::bool_query::BoolQuery;
use crate::query::constant_score_query::ConstantScoreQuery;
use crate::query::dis_max_query::DisjunctionMaxQuery;
use crate::query::fuzzy_query::FuzzyQuery;
use crate::query::match_query::MatchQuery;
use crate::query::multi_match_query::MultiMatchQuery;
use crate::query::phrase_query::PhraseQuery;
use crate::query::prefix_query::PrefixQuery;
use crate::query::range_query::RangeQuery;
use crate::query::term_query::TermQuery;

/// A structured search query.
///
/// Each variant represents a different kind of query constraint.
/// Queries can be nested inside `BoolQuery` and `DisjunctionMaxQuery`
/// to build complex search expressions.
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::Query;
/// use pelisearch_core::query::MatchQuery;
///
/// let q = Query::Match(MatchQuery::new("title", "electric bike"));
/// assert_eq!(
///     format!("{q:?}"),
///     "Match(MatchQuery { field: \"title\", value: \"electric bike\", term_boosts: {}, boost: 1.0 })"
/// );
/// ```
///
/// # Serialization
///
/// The enum is tagged with `"type"`:
///
/// ```json
/// {"type": "Match", "field": "title", "value": "electric bike"}
/// {"type": "Bool", "must": [...], "filter": [...]}
/// {"type": "Phrase", "field": "title", "value": "quick brown fox", "slop": 0}
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
    /// Boolean combination of sub-queries.
    Bool(BoolQuery),
    /// Multi-field match with per-field boosts.
    MultiMatch(MultiMatchQuery),
    /// Exact phrase with optional slop.
    Phrase(PhraseQuery),
    /// Fuzzy (Levenshtein) term matching.
    Fuzzy(FuzzyQuery),
    /// Prefix matching.
    Prefix(PrefixQuery),
    /// Wraps a query with a constant score.
    ConstantScore(ConstantScoreQuery),
    /// Disjunction max scoring across sub-queries.
    DisMax(DisjunctionMaxQuery),
    /// Matches all documents.
    MatchAll,
    /// Matches no documents.
    MatchNone,
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
