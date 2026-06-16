use serde::{Deserialize, Serialize};

use crate::query::query::Query;

/// A constant score query wraps another query and assigns a fixed score
/// to all matching documents. Useful for filters that should contribute
/// a uniform relevance boost.
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::{ConstantScoreQuery, Query, MatchQuery};
///
/// let q = ConstantScoreQuery::new(Query::Match(MatchQuery::new("title", "bike")), 2.0);
/// assert_eq!(q.score, 2.0);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstantScoreQuery {
    /// The inner query to execute.
    pub query: Box<Query>,
    /// The constant score to assign.
    #[serde(default = "default_constant_score")]
    pub score: f64,
}

fn default_constant_score() -> f64 {
    1.0
}

impl ConstantScoreQuery {
    pub fn new(query: Query, score: f64) -> Self {
        Self {
            query: Box::new(query),
            score,
        }
    }
}
