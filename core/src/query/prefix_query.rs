use serde::{Deserialize, Serialize};

use crate::tokenizer::tokenize;

/// A prefix query that matches documents containing terms that start
/// with a given prefix.
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::PrefixQuery;
///
/// let q = PrefixQuery::new("title", "bik");
/// assert_eq!(q.field, "title");
/// assert_eq!(q.value, "bik");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefixQuery {
    /// The field to search in.
    pub field: String,
    /// The prefix to match.
    pub value: String,
}

impl PrefixQuery {
    pub fn new(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
        }
    }

    /// Expand the prefix to all matching terms from the index.
    pub fn expand<'a>(
        &self,
        index_terms: impl IntoIterator<Item = &'a String>,
    ) -> Vec<String> {
        let prefix = tokenize(&self.value)
            .into_iter()
            .next()
            .unwrap_or_default();
        if prefix.is_empty() {
            return Vec::new();
        }
        index_terms
            .into_iter()
            .filter(|t| t.starts_with(&prefix))
            .cloned()
            .collect()
    }
}
