use serde::{Deserialize, Serialize};

/// A numeric range query.
///
/// Filters documents where the field value falls within the specified bounds.
/// All bounds are optional — at least one must be provided for a meaningful query.
///
/// # Examples
///
/// ```
/// use pelisearch_core::query::RangeQuery;
///
/// let q = RangeQuery::new("price")
///     .with_gte(10.0)
///     .with_lte(100.0);
/// assert_eq!(q.field, "price");
/// assert_eq!(q.gte, Some(10.0));
/// assert_eq!(q.lte, Some(100.0));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangeQuery {
    /// The numeric field to filter on.
    pub field: String,
    /// Values strictly greater than this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gt: Option<f64>,
    /// Values greater than or equal to this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gte: Option<f64>,
    /// Values strictly less than this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lt: Option<f64>,
    /// Values less than or equal to this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lte: Option<f64>,
}

impl RangeQuery {
    /// Create a new `RangeQuery` with no bounds set.
    ///
    /// Use the builder methods to add constraints.
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            gt: None,
            gte: None,
            lt: None,
            lte: None,
        }
    }

    /// Set the lower exclusive bound.
    pub fn with_gt(mut self, value: f64) -> Self {
        self.gt = Some(value);
        self
    }

    /// Set the lower inclusive bound.
    pub fn with_gte(mut self, value: f64) -> Self {
        self.gte = Some(value);
        self
    }

    /// Set the upper exclusive bound.
    pub fn with_lt(mut self, value: f64) -> Self {
        self.lt = Some(value);
        self
    }

    /// Set the upper inclusive bound.
    pub fn with_lte(mut self, value: f64) -> Self {
        self.lte = Some(value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_range_query_no_bounds() {
        let q = RangeQuery::new("price");
        assert_eq!(q.field, "price");
        assert_eq!(q.gt, None);
        assert_eq!(q.gte, None);
        assert_eq!(q.lt, None);
        assert_eq!(q.lte, None);
    }

    #[test]
    fn range_query_with_gte() {
        let q = RangeQuery::new("price").with_gte(50.0);
        assert_eq!(q.gte, Some(50.0));
        assert_eq!(q.lt, None);
    }

    #[test]
    fn range_query_with_lte() {
        let q = RangeQuery::new("price").with_lte(100.0);
        assert_eq!(q.lte, Some(100.0));
    }

    #[test]
    fn range_query_with_gt() {
        let q = RangeQuery::new("age").with_gt(18.0);
        assert_eq!(q.gt, Some(18.0));
    }

    #[test]
    fn range_query_with_lt() {
        let q = RangeQuery::new("age").with_lt(65.0);
        assert_eq!(q.lt, Some(65.0));
    }

    #[test]
    fn range_query_all_bounds() {
        let q = RangeQuery::new("price")
            .with_gte(10.0)
            .with_lte(100.0)
            .with_gt(9.0)
            .with_lt(101.0);
        assert_eq!(q.gte, Some(10.0));
        assert_eq!(q.lte, Some(100.0));
        assert_eq!(q.gt, Some(9.0));
        assert_eq!(q.lt, Some(101.0));
    }

    #[test]
    fn range_query_builder_chain() {
        let q = RangeQuery::new("rating").with_gte(3.5).with_lte(5.0);
        assert_eq!(q.field, "rating");
        assert_eq!(q.gte, Some(3.5));
        assert_eq!(q.lte, Some(5.0));
        assert_eq!(q.gt, None);
        assert_eq!(q.lt, None);
    }

    #[test]
    fn range_query_empty_field() {
        let q = RangeQuery::new("").with_gt(0.0);
        assert_eq!(q.field, "");
    }

    #[test]
    fn range_query_serde_roundtrip() {
        let q = RangeQuery::new("price").with_gte(10.0).with_lte(100.0);
        let json = serde_json::to_string(&q).unwrap();
        let deserialized: RangeQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q, deserialized);
    }

    #[test]
    fn range_query_serde_omits_none() {
        let q = RangeQuery::new("price").with_lte(100.0);
        let json = serde_json::to_string(&q).unwrap();
        assert!(!json.contains("\"gt\""), "json should not contain gt: {json}");
        assert!(!json.contains("\"gte\""), "json should not contain gte: {json}");
        assert!(!json.contains("\"lt\""), "json should not contain lt: {json}");
        assert!(json.contains("\"lte\""), "json should contain lte: {json}");
    }

    #[test]
    fn range_query_debug_output() {
        let q = RangeQuery::new("price").with_gte(1.0).with_lte(10.0);
        let debug = format!("{q:?}");
        assert!(debug.contains("price"));
        assert!(debug.contains("1.0"));
        assert!(debug.contains("10.0"));
    }

    #[test]
    fn range_query_clone() {
        let a = RangeQuery::new("x").with_gte(1.0);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn range_query_partial_eq() {
        let a = RangeQuery::new("p").with_gte(1.0);
        let b = RangeQuery::new("p").with_gte(1.0);
        let c = RangeQuery::new("p").with_gte(2.0);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
