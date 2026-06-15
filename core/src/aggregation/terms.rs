use serde::{Deserialize, Serialize};

/// A terms aggregation that buckets documents by distinct field values.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::TermsAggregation;
///
/// let agg = TermsAggregation::new("category").with_size(10);
/// assert_eq!(agg.field, "category");
/// assert_eq!(agg.size, Some(10));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TermsAggregation {
    /// The field to bucket by.
    pub field: String,
    /// Maximum number of buckets to return (default: unlimited).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
}

impl TermsAggregation {
    /// Create a new `TermsAggregation` for the given field.
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            size: None,
        }
    }

    /// Set the maximum number of buckets to return.
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = Some(size);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_terms_aggregation() {
        let agg = TermsAggregation::new("category");
        assert_eq!(agg.field, "category");
        assert_eq!(agg.size, None);
    }

    #[test]
    fn terms_with_size() {
        let agg = TermsAggregation::new("category").with_size(5);
        assert_eq!(agg.size, Some(5));
    }

    #[test]
    fn terms_empty_field() {
        let agg = TermsAggregation::new("");
        assert_eq!(agg.field, "");
    }

    #[test]
    fn terms_serde_roundtrip() {
        let agg = TermsAggregation::new("status").with_size(10);
        let json = serde_json::to_string(&agg).unwrap();
        let deserialized: TermsAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(agg, deserialized);
    }

    #[test]
    fn terms_serde_omits_size_when_none() {
        let agg = TermsAggregation::new("cat");
        let json = serde_json::to_string(&agg).unwrap();
        assert!(!json.contains("size"));
    }

    #[test]
    fn terms_debug_output() {
        let agg = TermsAggregation::new("x").with_size(3);
        let debug = format!("{agg:?}");
        assert!(debug.contains("x"));
    }

    #[test]
    fn terms_clone() {
        let a = TermsAggregation::new("f").with_size(2);
        let b = a.clone();
        assert_eq!(a, b);
    }
}
