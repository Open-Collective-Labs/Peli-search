use serde::{Deserialize, Serialize};

use crate::aggregation::metrics::{
    AverageAggregation, CountAggregation, MaxAggregation, MinAggregation, SumAggregation,
};
use crate::aggregation::terms::TermsAggregation;

/// A named aggregation specification.
///
/// Aggregations compute summary metrics or bucket documents by field values
/// after query execution.
///
/// # Examples
///
/// ```
/// use pelisearch_core::aggregation::{Aggregation, TermsAggregation, CountAggregation};
///
/// let terms = Aggregation::Terms(TermsAggregation::new("category").with_size(10));
/// let count = Aggregation::Count(CountAggregation::new("price"));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Aggregation {
    /// Bucket documents by field value.
    Terms(TermsAggregation),
    /// Count of documents with a non-null field value.
    Count(CountAggregation),
    /// Minimum value of a numeric field.
    Min(MinAggregation),
    /// Maximum value of a numeric field.
    Max(MaxAggregation),
    /// Average value of a numeric field.
    Average(AverageAggregation),
    /// Sum of values of a numeric field.
    Sum(SumAggregation),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_terms_aggregation() {
        let agg = Aggregation::Terms(TermsAggregation::new("category"));
        assert!(matches!(agg, Aggregation::Terms(_)));
    }

    #[test]
    fn create_count_aggregation() {
        let agg = Aggregation::Count(CountAggregation::new("price"));
        assert!(matches!(agg, Aggregation::Count(_)));
    }

    #[test]
    fn create_min_aggregation() {
        let agg = Aggregation::Min(MinAggregation::new("price"));
        assert!(matches!(agg, Aggregation::Min(_)));
    }

    #[test]
    fn create_max_aggregation() {
        let agg = Aggregation::Max(MaxAggregation::new("price"));
        assert!(matches!(agg, Aggregation::Max(_)));
    }

    #[test]
    fn create_average_aggregation() {
        let agg = Aggregation::Average(AverageAggregation::new("price"));
        assert!(matches!(agg, Aggregation::Average(_)));
    }

    #[test]
    fn create_sum_aggregation() {
        let agg = Aggregation::Sum(SumAggregation::new("price"));
        assert!(matches!(agg, Aggregation::Sum(_)));
    }

    #[test]
    fn aggregation_serde_roundtrip() {
        let agg = Aggregation::Terms(TermsAggregation::new("category").with_size(5));
        let json = serde_json::to_string(&agg).unwrap();
        let deserialized: Aggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(agg, deserialized);
    }

    #[test]
    fn aggregation_variants_are_distinct() {
        let a = Aggregation::Count(CountAggregation::new("f"));
        let b = Aggregation::Min(MinAggregation::new("f"));
        assert_ne!(a, b);
    }

    #[test]
    fn aggregation_debug_output() {
        let agg = Aggregation::Terms(TermsAggregation::new("cat"));
        let debug = format!("{agg:?}");
        assert!(debug.contains("cat"));
    }

    #[test]
    fn aggregation_clone() {
        let a = Aggregation::Sum(SumAggregation::new("f"));
        let b = a.clone();
        assert_eq!(a, b);
    }
}
