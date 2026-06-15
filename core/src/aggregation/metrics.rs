use serde::{Deserialize, Serialize};

/// Count aggregation — count documents with a non-null field value.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::CountAggregation;
///
/// let agg = CountAggregation::new("price");
/// assert_eq!(agg.field, "price");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CountAggregation {
    /// The field to count non-null values for.
    pub field: String,
}

impl CountAggregation {
    /// Create a new `CountAggregation`.
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
        }
    }
}

/// Min aggregation — minimum value of a numeric field.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::MinAggregation;
///
/// let agg = MinAggregation::new("price");
/// assert_eq!(agg.field, "price");
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
}

/// Max aggregation — maximum value of a numeric field.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::MaxAggregation;
///
/// let agg = MaxAggregation::new("price");
/// assert_eq!(agg.field, "price");
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
}

/// Average aggregation — mean value of a numeric field.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::AverageAggregation;
///
/// let agg = AverageAggregation::new("price");
/// assert_eq!(agg.field, "price");
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
}

/// Sum aggregation — total of all numeric field values.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::SumAggregation;
///
/// let agg = SumAggregation::new("price");
/// assert_eq!(agg.field, "price");
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_aggregation() {
        let agg = CountAggregation::new("price");
        assert_eq!(agg.field, "price");
    }

    #[test]
    fn min_aggregation() {
        let agg = MinAggregation::new("price");
        assert_eq!(agg.field, "price");
    }

    #[test]
    fn max_aggregation() {
        let agg = MaxAggregation::new("price");
        assert_eq!(agg.field, "price");
    }

    #[test]
    fn average_aggregation() {
        let agg = AverageAggregation::new("price");
        assert_eq!(agg.field, "price");
    }

    #[test]
    fn sum_aggregation() {
        let agg = SumAggregation::new("price");
        assert_eq!(agg.field, "price");
    }

    #[test]
    fn empty_field() {
        let agg = CountAggregation::new("");
        assert_eq!(agg.field, "");
    }

    #[test]
    fn serde_roundtrip() {
        let agg = AverageAggregation::new("rating");
        let json = serde_json::to_string(&agg).unwrap();
        let deserialized: AverageAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(agg, deserialized);
    }

    #[test]
    fn debug_output() {
        let agg = MinAggregation::new("temp");
        let debug = format!("{agg:?}");
        assert!(debug.contains("temp"));
    }

    #[test]
    fn clone() {
        let a = MaxAggregation::new("x");
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn partial_eq() {
        let a = SumAggregation::new("a");
        let b = SumAggregation::new("a");
        let c = SumAggregation::new("b");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
