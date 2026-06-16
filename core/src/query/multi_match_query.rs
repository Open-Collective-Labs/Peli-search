use serde::{Deserialize, Serialize};

/// A query that searches the same text across multiple fields with
/// optional per-field boosts.
///
/// Results are scored using disjunction max (max + tie_breaker * rest).
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::MultiMatchQuery;
///
/// let q = MultiMatchQuery::new("bike")
///     .field("title", 2.0)
///     .field("description", 1.0);
/// assert_eq!(q.fields.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiMatchQuery {
    /// The text to search for (analyzed).
    pub value: String,
    /// Fields with optional boosts. (field_name, boost)
    pub fields: Vec<(String, f32)>,
    /// Tie-breaker for dis_max combination (default: 0.0).
    /// 0.0 = max only, 1.0 = sum.
    #[serde(default)]
    pub tie_breaker: f32,
}

impl MultiMatchQuery {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            fields: Vec::new(),
            tie_breaker: 0.0,
        }
    }

    pub fn field(mut self, field: impl Into<String>, boost: f32) -> Self {
        self.fields.push((field.into(), boost));
        self
    }

    pub fn tie_breaker(mut self, tb: f32) -> Self {
        self.tie_breaker = tb;
        self
    }
}
