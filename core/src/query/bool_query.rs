use serde::{Deserialize, Serialize};

use crate::query::query::Query;

/// A boolean query combining multiple sub-queries with boolean logic.
///
/// - `must`: clauses that must match (contribute to score)
/// - `should`: clauses that should match (boost score, become required if no `must`)
/// - `filter`: clauses that must match (no score contribution)
/// - `must_not`: clauses that must not match
/// - `minimum_should_match`: minimum number of `should` clauses that must match
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::{BoolQuery, Query, MatchQuery, TermQuery};
///
/// let q = BoolQuery::new()
///     .must(Query::Match(MatchQuery::new("title", "bike")))
///     .filter(Query::Term(TermQuery::new("category", "electronics")));
/// assert_eq!(q.must.len(), 1);
/// assert_eq!(q.filter.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoolQuery {
    /// Clauses that must match (scored).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must: Vec<Query>,
    /// Clauses that should match.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub should: Vec<Query>,
    /// Clauses that must match (no score contribution).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filter: Vec<Query>,
    /// Clauses that must not match.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_not: Vec<Query>,
    /// Minimum number of should clauses that must match.
    /// When 0, should clauses are optional unless there are no must/filter clauses.
    #[serde(default)]
    pub minimum_should_match: usize,
}

impl BoolQuery {
    pub fn new() -> Self {
        Self {
            must: Vec::new(),
            should: Vec::new(),
            filter: Vec::new(),
            must_not: Vec::new(),
            minimum_should_match: 0,
        }
    }

    pub fn must(mut self, q: Query) -> Self {
        self.must.push(q);
        self
    }

    pub fn should(mut self, q: Query) -> Self {
        self.should.push(q);
        self
    }

    pub fn filter(mut self, q: Query) -> Self {
        self.filter.push(q);
        self
    }

    pub fn must_not(mut self, q: Query) -> Self {
        self.must_not.push(q);
        self
    }

    pub fn minimum_should_match(mut self, n: usize) -> Self {
        self.minimum_should_match = n;
        self
    }
}

impl Default for BoolQuery {
    fn default() -> Self {
        Self::new()
    }
}
