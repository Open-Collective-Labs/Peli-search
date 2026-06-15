use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::document::Document;

/// A single bucket produced by a terms aggregation.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::TermsBucket;
///
/// let bucket = TermsBucket::new("electronics", 5);
/// assert_eq!(bucket.key, "electronics");
/// assert_eq!(bucket.count, 5);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TermsBucket {
    /// The distinct field value.
    pub key: String,
    /// Number of documents with this value.
    pub count: usize,
}

impl TermsBucket {
    /// Create a new `TermsBucket`.
    pub fn new(key: impl Into<String>, count: usize) -> Self {
        Self {
            key: key.into(),
            count,
        }
    }
}

/// A terms aggregation that buckets documents by distinct field values.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::document::Document;
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

    /// Execute this aggregation against a slice of documents.
    ///
    /// Counts occurrences of each distinct string value for the configured
    /// field. Buckets are sorted by count descending (highest first).
    /// Missing fields and non-string values are ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::aggregation::TermsAggregation;
    ///
    /// let agg = TermsAggregation::new("category");
    ///
    /// let docs = vec![
    ///     Document::new("a", HashMap::from([("category".into(), serde_json::json!("bike"))])).unwrap(),
    ///     Document::new("b", HashMap::from([("category".into(), serde_json::json!("bike"))])).unwrap(),
    ///     Document::new("c", HashMap::from([("category".into(), serde_json::json!("helmet"))])).unwrap(),
    /// ];
    ///
    /// let buckets = agg.execute(&docs);
    /// assert_eq!(buckets.len(), 2);
    /// assert_eq!(buckets[0].key, "bike");
    /// assert_eq!(buckets[0].count, 2);
    /// assert_eq!(buckets[1].key, "helmet");
    /// assert_eq!(buckets[1].count, 1);
    /// ```
    pub fn execute(&self, documents: &[Document]) -> Vec<TermsBucket> {
        let mut counts: HashMap<String, usize> = HashMap::new();

        for doc in documents {
            if let Some(serde_json::Value::String(val)) = doc.get_field(&self.field) {
                *counts.entry(val.clone()).or_insert(0) += 1;
            }
        }

        let mut buckets: Vec<TermsBucket> = counts
            .into_iter()
            .map(|(key, count)| TermsBucket::new(key, count))
            .collect();

        buckets.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));

        if let Some(limit) = self.size {
            buckets.truncate(limit);
        }

        buckets
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;

    use super::*;

    fn doc(category: &str) -> Document {
        let mut fields = HashMap::new();
        fields.insert(
            "category".to_string(),
            serde_json::json!(category),
        );
        Document::new("doc", fields).unwrap()
    }

    fn doc_missing() -> Document {
        Document::new("doc", HashMap::new()).unwrap()
    }

    fn doc_numeric() -> Document {
        let mut fields = HashMap::new();
        fields.insert("category".to_string(), serde_json::json!(42));
        Document::new("doc", fields).unwrap()
    }

    #[test]
    fn correct_counts() {
        let agg = TermsAggregation::new("category");
        let docs = vec![doc("bike"), doc("bike"), doc("helmet")];
        let buckets = agg.execute(&docs);
        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[0].key, "bike");
        assert_eq!(buckets[0].count, 2);
        assert_eq!(buckets[1].key, "helmet");
        assert_eq!(buckets[1].count, 1);
    }

    #[test]
    fn empty_documents_returns_empty() {
        let agg = TermsAggregation::new("category");
        let buckets = agg.execute(&[]);
        assert!(buckets.is_empty());
    }

    #[test]
    fn missing_field_ignored() {
        let agg = TermsAggregation::new("category");
        let docs = vec![doc_missing()];
        let buckets = agg.execute(&docs);
        assert!(buckets.is_empty());
    }

    #[test]
    fn non_string_value_ignored() {
        let agg = TermsAggregation::new("category");
        let docs = vec![doc_numeric()];
        let buckets = agg.execute(&docs);
        assert!(buckets.is_empty());
    }

    #[test]
    fn mixed_missing_and_present() {
        let agg = TermsAggregation::new("category");
        let docs = vec![doc("bike"), doc_missing(), doc("bike")];
        let buckets = agg.execute(&docs);
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].key, "bike");
        assert_eq!(buckets[0].count, 2);
    }

    #[test]
    fn size_limit_respected() {
        let agg = TermsAggregation::new("category").with_size(2);
        let docs = vec![
            doc("a"), doc("a"), doc("a"),
            doc("b"), doc("b"),
            doc("c"),
        ];
        let buckets = agg.execute(&docs);
        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[0].key, "a");
        assert_eq!(buckets[0].count, 3);
        assert_eq!(buckets[1].key, "b");
        assert_eq!(buckets[1].count, 2);
    }

    #[test]
    fn size_zero_returns_empty() {
        let agg = TermsAggregation::new("category").with_size(0);
        let docs = vec![doc("bike")];
        let buckets = agg.execute(&docs);
        assert!(buckets.is_empty());
    }

    #[test]
    fn sorted_by_count_descending() {
        let agg = TermsAggregation::new("category");
        let docs = vec![
            doc("z"), doc("z"),
            doc("a"), doc("a"), doc("a"),
            doc("m"),
        ];
        let buckets = agg.execute(&docs);
        assert_eq!(buckets[0].key, "a"); // count 3
        assert_eq!(buckets[1].key, "z"); // count 2
        assert_eq!(buckets[2].key, "m"); // count 1
    }

    #[test]
    fn tiebreaker_by_key() {
        let agg = TermsAggregation::new("category");
        let docs = vec![doc("b"), doc("a")];
        let buckets = agg.execute(&docs);
        // Both count 1, tiebroken by key asc
        assert_eq!(buckets[0].key, "a");
        assert_eq!(buckets[1].key, "b");
    }

    #[test]
    fn terms_bucket_creation() {
        let b = TermsBucket::new("cat", 3);
        assert_eq!(b.key, "cat");
        assert_eq!(b.count, 3);
    }

    #[test]
    fn terms_bucket_serde() {
        let b = TermsBucket::new("x", 5);
        let json = serde_json::to_string(&b).unwrap();
        let deserialized: TermsBucket = serde_json::from_str(&json).unwrap();
        assert_eq!(b, deserialized);
    }
}
