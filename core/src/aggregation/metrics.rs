use serde::{Deserialize, Serialize};

use crate::document::Document;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Result of a count aggregation.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::CountResult;
///
/// let r = CountResult { count: 5 };
/// assert_eq!(r.count, 5);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CountResult {
    /// Number of documents with a numeric value for the field.
    pub count: usize,
}

/// Result of a min aggregation.
///
/// `value` is `None` when no document has a numeric value for the field.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::MinResult;
///
/// let r = MinResult { value: Some(10.0) };
/// assert_eq!(r.value, Some(10.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MinResult {
    /// The minimum value, or `None` if no valid values.
    pub value: Option<f64>,
}

/// Result of a max aggregation.
///
/// `value` is `None` when no document has a numeric value for the field.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::MaxResult;
///
/// let r = MaxResult { value: Some(100.0) };
/// assert_eq!(r.value, Some(100.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MaxResult {
    /// The maximum value, or `None` if no valid values.
    pub value: Option<f64>,
}

/// Result of an average aggregation.
///
/// `value` is `None` when no document has a numeric value for the field.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::AverageResult;
///
/// let r = AverageResult { value: Some(50.0) };
/// assert_eq!(r.value, Some(50.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AverageResult {
    /// The average value, or `None` if no valid values.
    pub value: Option<f64>,
}

/// Result of a sum aggregation.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::SumResult;
///
/// let r = SumResult { value: 250.0 };
/// assert_eq!(r.value, 250.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SumResult {
    /// The sum of all numeric values.
    pub value: f64,
}

// ---------------------------------------------------------------------------
// Aggregation types
// ---------------------------------------------------------------------------

/// Count aggregation — count documents with a numeric field value.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::document::Document;
/// use pelisearch_core::aggregation::CountAggregation;
///
/// let agg = CountAggregation::new("price");
/// let doc = Document::new("a", HashMap::from([("price".into(), serde_json::json!(10.0))])).unwrap();
/// let result = agg.execute(&[doc]);
/// assert_eq!(result.count, 1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CountAggregation {
    /// The field to count numeric values for.
    pub field: String,
}

impl CountAggregation {
    /// Create a new `CountAggregation`.
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
        }
    }

    /// Execute this aggregation against a slice of documents.
    ///
    /// Counts documents that have a numeric value for the configured field.
    pub fn execute(&self, documents: &[Document]) -> CountResult {
        let count = documents
            .iter()
            .filter(|doc| is_numeric_field(doc, &self.field))
            .count();
        CountResult { count }
    }
}

/// Min aggregation — minimum value of a numeric field.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::document::Document;
/// use pelisearch_core::aggregation::MinAggregation;
///
/// let agg = MinAggregation::new("price");
/// let docs = vec![
///     Document::new("a", HashMap::from([("price".into(), serde_json::json!(50.0))])).unwrap(),
///     Document::new("b", HashMap::from([("price".into(), serde_json::json!(10.0))])).unwrap(),
/// ];
/// let result = agg.execute(&docs);
/// assert_eq!(result.value, Some(10.0));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MinAggregation {
    /// The numeric field to find the minimum value of.
    pub field: String,
}

impl MinAggregation {
    /// Create a new `MinAggregation`.
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
        }
    }

    /// Execute this aggregation against a slice of documents.
    ///
    /// Returns the minimum numeric value, or `None` if no document has a
    /// numeric value for the field.
    pub fn execute(&self, documents: &[Document]) -> MinResult {
        let value = documents
            .iter()
            .filter_map(|doc| extract_numeric(doc, &self.field))
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        MinResult { value }
    }
}

/// Max aggregation — maximum value of a numeric field.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::document::Document;
/// use pelisearch_core::aggregation::MaxAggregation;
///
/// let agg = MaxAggregation::new("price");
/// let docs = vec![
///     Document::new("a", HashMap::from([("price".into(), serde_json::json!(50.0))])).unwrap(),
///     Document::new("b", HashMap::from([("price".into(), serde_json::json!(100.0))])).unwrap(),
/// ];
/// let result = agg.execute(&docs);
/// assert_eq!(result.value, Some(100.0));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaxAggregation {
    /// The numeric field to find the maximum value of.
    pub field: String,
}

impl MaxAggregation {
    /// Create a new `MaxAggregation`.
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
        }
    }

    /// Execute this aggregation against a slice of documents.
    ///
    /// Returns the maximum numeric value, or `None` if no document has a
    /// numeric value for the field.
    pub fn execute(&self, documents: &[Document]) -> MaxResult {
        let value = documents
            .iter()
            .filter_map(|doc| extract_numeric(doc, &self.field))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        MaxResult { value }
    }
}

/// Average aggregation — mean value of a numeric field.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::document::Document;
/// use pelisearch_core::aggregation::AverageAggregation;
///
/// let agg = AverageAggregation::new("price");
/// let docs = vec![
///     Document::new("a", HashMap::from([("price".into(), serde_json::json!(10.0))])).unwrap(),
///     Document::new("b", HashMap::from([("price".into(), serde_json::json!(20.0))])).unwrap(),
/// ];
/// let result = agg.execute(&docs);
/// assert!((result.value.unwrap() - 15.0).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AverageAggregation {
    /// The numeric field to compute the average of.
    pub field: String,
}

impl AverageAggregation {
    /// Create a new `AverageAggregation`.
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
        }
    }

    /// Execute this aggregation against a slice of documents.
    ///
    /// Returns the arithmetic mean of all numeric values, or `None` if no
    /// document has a numeric value for the field.
    pub fn execute(&self, documents: &[Document]) -> AverageResult {
        let values: Vec<f64> = documents
            .iter()
            .filter_map(|doc| extract_numeric(doc, &self.field))
            .collect();

        if values.is_empty() {
            return AverageResult { value: None };
        }

        let sum: f64 = values.iter().sum();
        let avg = sum / values.len() as f64;
        AverageResult { value: Some(avg) }
    }
}

/// Sum aggregation — total of all numeric field values.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::document::Document;
/// use pelisearch_core::aggregation::SumAggregation;
///
/// let agg = SumAggregation::new("price");
/// let docs = vec![
///     Document::new("a", HashMap::from([("price".into(), serde_json::json!(10.0))])).unwrap(),
///     Document::new("b", HashMap::from([("price".into(), serde_json::json!(20.0))])).unwrap(),
/// ];
/// let result = agg.execute(&docs);
/// assert!((result.value - 30.0).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SumAggregation {
    /// The numeric field to sum.
    pub field: String,
}

impl SumAggregation {
    /// Create a new `SumAggregation`.
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
        }
    }

    /// Execute this aggregation against a slice of documents.
    ///
    /// Returns the sum of all numeric values (0.0 if no valid values).
    pub fn execute(&self, documents: &[Document]) -> SumResult {
        let sum: f64 = documents
            .iter()
            .filter_map(|doc| extract_numeric(doc, &self.field))
            .sum();
        SumResult { value: sum }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn extract_numeric(doc: &Document, field: &str) -> Option<f64> {
    match doc.get_field(field) {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        _ => None,
    }
}

fn is_numeric_field(doc: &Document, field: &str) -> bool {
    extract_numeric(doc, field).is_some()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;

    use super::*;

    fn price_doc(id: &str, price: f64) -> Document {
        let mut fields = HashMap::new();
        fields.insert("price".to_string(), serde_json::json!(price));
        Document::new(id, fields).unwrap()
    }

    fn missing_doc(id: &str) -> Document {
        Document::new(id, HashMap::new()).unwrap()
    }

    fn string_doc(id: &str) -> Document {
        let mut fields = HashMap::new();
        fields.insert("price".to_string(), serde_json::json!("not_a_number"));
        Document::new(id, fields).unwrap()
    }

    // --- Count ---

    #[test]
    fn count_all_numeric() {
        let agg = CountAggregation::new("price");
        let docs = vec![price_doc("a", 1.0), price_doc("b", 2.0), price_doc("c", 3.0)];
        assert_eq!(agg.execute(&docs).count, 3);
    }

    #[test]
    fn count_empty() {
        let agg = CountAggregation::new("price");
        assert_eq!(agg.execute(&[]).count, 0);
    }

    #[test]
    fn count_missing_field() {
        let agg = CountAggregation::new("price");
        let docs = vec![missing_doc("a")];
        assert_eq!(agg.execute(&docs).count, 0);
    }

    #[test]
    fn count_non_numeric() {
        let agg = CountAggregation::new("price");
        let docs = vec![string_doc("a")];
        assert_eq!(agg.execute(&docs).count, 0);
    }

    #[test]
    fn count_mixed() {
        let agg = CountAggregation::new("price");
        let docs = vec![price_doc("a", 1.0), missing_doc("b"), string_doc("c")];
        assert_eq!(agg.execute(&docs).count, 1);
    }

    // --- Min ---

    #[test]
    fn min_basic() {
        let agg = MinAggregation::new("price");
        let docs = vec![price_doc("a", 50.0), price_doc("b", 10.0), price_doc("c", 30.0)];
        assert_eq!(agg.execute(&docs).value, Some(10.0));
    }

    #[test]
    fn min_empty() {
        let agg = MinAggregation::new("price");
        assert_eq!(agg.execute(&[]).value, None);
    }

    #[test]
    fn min_missing_field() {
        let agg = MinAggregation::new("price");
        assert_eq!(agg.execute(&[missing_doc("a")]).value, None);
    }

    #[test]
    fn min_non_numeric() {
        let agg = MinAggregation::new("price");
        assert_eq!(agg.execute(&[string_doc("a")]).value, None);
    }

    #[test]
    fn min_negative_values() {
        let agg = MinAggregation::new("price");
        let docs = vec![price_doc("a", -10.0), price_doc("b", -5.0)];
        assert_eq!(agg.execute(&docs).value, Some(-10.0));
    }

    // --- Max ---

    #[test]
    fn max_basic() {
        let agg = MaxAggregation::new("price");
        let docs = vec![price_doc("a", 50.0), price_doc("b", 100.0), price_doc("c", 30.0)];
        assert_eq!(agg.execute(&docs).value, Some(100.0));
    }

    #[test]
    fn max_empty() {
        let agg = MaxAggregation::new("price");
        assert_eq!(agg.execute(&[]).value, None);
    }

    #[test]
    fn max_missing_field() {
        let agg = MaxAggregation::new("price");
        assert_eq!(agg.execute(&[missing_doc("a")]).value, None);
    }

    #[test]
    fn max_non_numeric() {
        let agg = MaxAggregation::new("price");
        assert_eq!(agg.execute(&[string_doc("a")]).value, None);
    }

    #[test]
    fn max_negative_values() {
        let agg = MaxAggregation::new("price");
        let docs = vec![price_doc("a", -10.0), price_doc("b", -5.0)];
        assert_eq!(agg.execute(&docs).value, Some(-5.0));
    }

    // --- Average ---

    #[test]
    fn average_basic() {
        let agg = AverageAggregation::new("price");
        let docs = vec![price_doc("a", 10.0), price_doc("b", 20.0), price_doc("c", 30.0)];
        let result = agg.execute(&docs);
        assert!((result.value.unwrap() - 20.0).abs() < 1e-10);
    }

    #[test]
    fn average_single_value() {
        let agg = AverageAggregation::new("price");
        let docs = vec![price_doc("a", 42.0)];
        let result = agg.execute(&docs);
        assert!((result.value.unwrap() - 42.0).abs() < 1e-10);
    }

    #[test]
    fn average_empty() {
        let agg = AverageAggregation::new("price");
        assert_eq!(agg.execute(&[]).value, None);
    }

    #[test]
    fn average_missing_field() {
        let agg = AverageAggregation::new("price");
        assert_eq!(agg.execute(&[missing_doc("a")]).value, None);
    }

    #[test]
    fn average_non_numeric() {
        let agg = AverageAggregation::new("price");
        assert_eq!(agg.execute(&[string_doc("a")]).value, None);
    }

    #[test]
    fn average_negative_values() {
        let agg = AverageAggregation::new("price");
        let docs = vec![price_doc("a", -10.0), price_doc("b", -20.0)];
        let result = agg.execute(&docs);
        assert!((result.value.unwrap() - (-15.0)).abs() < 1e-10);
    }

    #[test]
    fn average_with_missing_and_valid() {
        let agg = AverageAggregation::new("price");
        let docs = vec![price_doc("a", 10.0), missing_doc("b")];
        let result = agg.execute(&docs);
        assert!((result.value.unwrap() - 10.0).abs() < 1e-10);
    }

    // --- Sum ---

    #[test]
    fn sum_basic() {
        let agg = SumAggregation::new("price");
        let docs = vec![price_doc("a", 100.0), price_doc("b", 200.0)];
        let result = agg.execute(&docs);
        assert!((result.value - 300.0).abs() < 1e-10);
    }

    #[test]
    fn sum_empty() {
        let agg = SumAggregation::new("price");
        assert!((agg.execute(&[]).value - 0.0).abs() < 1e-10);
    }

    #[test]
    fn sum_missing_field() {
        let agg = SumAggregation::new("price");
        assert!((agg.execute(&[missing_doc("a")]).value - 0.0).abs() < 1e-10);
    }

    #[test]
    fn sum_non_numeric() {
        let agg = SumAggregation::new("price");
        assert!((agg.execute(&[string_doc("a")]).value - 0.0).abs() < 1e-10);
    }

    #[test]
    fn sum_negative_values() {
        let agg = SumAggregation::new("price");
        let docs = vec![price_doc("a", -50.0), price_doc("b", 30.0)];
        assert!((agg.execute(&docs).value - (-20.0)).abs() < 1e-10);
    }

    #[test]
    fn sum_mixed() {
        let agg = SumAggregation::new("price");
        let docs = vec![price_doc("a", 10.0), missing_doc("b"), string_doc("c")];
        assert!((agg.execute(&docs).value - 10.0).abs() < 1e-10);
    }

    // --- Result types ---

    #[test]
    fn count_result_serde() {
        let r = CountResult { count: 5 };
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, "{\"count\":5}");
    }

    #[test]
    fn min_result_serde() {
        let r = MinResult { value: Some(1.5) };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("1.5"));
    }

    #[test]
    fn min_result_none_serde() {
        let r = MinResult { value: None };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("null"));
    }

    #[test]
    fn max_result_serde() {
        let r = MaxResult { value: Some(99.0) };
        let json = serde_json::to_string(&r).unwrap();
        let deserialized: MaxResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r, deserialized);
    }

    #[test]
    fn average_result_serde() {
        let r = AverageResult { value: Some(50.5) };
        let json = serde_json::to_string(&r).unwrap();
        let deserialized: AverageResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r, deserialized);
    }

    #[test]
    fn sum_result_serde() {
        let r = SumResult { value: 123.45 };
        let json = serde_json::to_string(&r).unwrap();
        let deserialized: SumResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r, deserialized);
    }
}
