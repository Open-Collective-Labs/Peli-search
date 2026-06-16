use serde::{Deserialize, Serialize};

use crate::tokenizer::tokenize;

/// A fuzzy query that matches terms similar to the input using
/// Levenshtein (edit distance) automaton.
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::FuzzyQuery;
///
/// let q = FuzzyQuery::new("title", "bike").max_edits(2);
/// assert_eq!(q.value, "bike");
/// assert_eq!(q.max_edit_distance, 2);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuzzyQuery {
    /// The field to search in.
    pub field: String,
    /// The approximate text to match.
    pub value: String,
    /// Maximum Levenshtein edit distance (0-2, default: 2).
    #[serde(default = "default_max_edits")]
    pub max_edit_distance: u8,
    /// Number of initial characters that must match exactly (default: 0).
    #[serde(default)]
    pub prefix_length: u8,
}

const fn default_max_edits() -> u8 {
    2
}

impl FuzzyQuery {
    pub fn new(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
            max_edit_distance: 2,
            prefix_length: 0,
        }
    }

    pub fn max_edits(mut self, n: u8) -> Self {
        self.max_edit_distance = n.min(2);
        self
    }

    pub fn prefix_length(mut self, n: u8) -> Self {
        self.prefix_length = n;
        self
    }

    /// Compute Levenshtein distance between two strings.
    pub fn levenshtein(a: &str, b: &str) -> u8 {
        let a = a.as_bytes();
        let b = b.as_bytes();
        let a_len = a.len();
        let b_len = b.len();

        if a_len == 0 {
            return b_len as u8;
        }
        if b_len == 0 {
            return a_len as u8;
        }

        let mut prev: Vec<usize> = (0..=b_len).collect();
        let mut curr = vec![0usize; b_len + 1];

        for i in 0..a_len {
            curr[0] = i + 1;
            for j in 0..b_len {
                let cost = if a[i] == b[j] { 0 } else { 1 };
                curr[j + 1] = std::cmp::min(
                    std::cmp::min(curr[j] + 1, prev[j + 1] + 1),
                    prev[j] + cost,
                );
            }
            std::mem::swap(&mut prev, &mut curr);
        }

        prev[b_len] as u8
    }

    /// Get the query terms after fuzzy expansion.
    /// Returns (original_tokens, expanded_terms) where expanded_terms
    /// is a flat list of all index terms within edit distance.
    pub fn expand_terms<'a>(
        &self,
        index_terms: impl IntoIterator<Item = &'a String>,
    ) -> Vec<(String, String)> {
        let tokens = tokenize(&self.value);
        let mut expanded = Vec::new();

        let terms: Vec<&String> = index_terms.into_iter().collect();

        for token in &tokens {
            let prefix_len = self.prefix_length.min(token.len() as u8) as usize;
            let prefix = &token[..prefix_len];
            for term in &terms {
                if !term.starts_with(prefix) {
                    continue;
                }
                let dist = Self::levenshtein(token, term);
                if dist <= self.max_edit_distance {
                    expanded.push((token.clone(), (*term).clone()));
                }
            }
        }

        expanded
    }
}
