use serde::{Deserialize, Serialize};

/// A phrase query that matches documents containing the exact phrase
/// (or within a slop distance).
///
/// The `value` is tokenized and all tokens must appear in the document
/// in the same order, within `slop` positions of each other.
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::PhraseQuery;
///
/// let q = PhraseQuery::new("title", "quick brown fox");
/// assert_eq!(q.field, "title");
/// assert!(q.slop == 0);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhraseQuery {
    /// The field to search in.
    pub field: String,
    /// The phrase text to match (analyzed).
    pub value: String,
    /// Maximum position gap between consecutive terms (default: 0, meaning exact).
    #[serde(default)]
    pub slop: usize,
}

impl PhraseQuery {
    pub fn new(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
            slop: 0,
        }
    }

    pub fn slop(mut self, slop: usize) -> Self {
        self.slop = slop;
        self
    }
}
