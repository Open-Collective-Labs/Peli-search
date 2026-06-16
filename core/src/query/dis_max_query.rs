use serde::{Deserialize, Serialize};

use crate::query::query::Query;

/// A disjunction max query that returns documents matching any of the
/// sub-queries, scoring them as `max + tie_breaker * (sum - max)`.
///
/// This produces smoother relevance than plain OR when multiple
/// sub-queries match the same document.
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::{DisjunctionMaxQuery, Query, MatchQuery};
///
/// let q = DisjunctionMaxQuery::new()
///     .query(Query::Match(MatchQuery::new("title", "bike")))
///     .query(Query::Match(MatchQuery::new("description", "bicycle")))
///     .tie_breaker(0.3);
/// assert_eq!(q.queries.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisjunctionMaxQuery {
    /// Sub-queries to combine.
    pub queries: Vec<Query>,
    /// Tie-breaker multiplier (0.0 = max only, 1.0 = sum).
    #[serde(default)]
    pub tie_breaker: f32,
}

impl DisjunctionMaxQuery {
    pub fn new() -> Self {
        Self {
            queries: Vec::new(),
            tie_breaker: 0.0,
        }
    }

    pub fn query(mut self, q: Query) -> Self {
        self.queries.push(q);
        self
    }

    pub fn tie_breaker(mut self, tb: f32) -> Self {
        self.tie_breaker = tb;
        self
    }
}

impl Default for DisjunctionMaxQuery {
    fn default() -> Self {
        Self::new()
    }
}
