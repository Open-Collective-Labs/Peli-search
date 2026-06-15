use crate::document::Document;
use crate::filter::evaluator::FilterEvaluator;
use crate::query::RangeQuery;

impl FilterEvaluator for RangeQuery {
    /// Returns `true` when the document has the specified field and its value
    /// is a JSON number that falls within the configured bounds.
    ///
    /// If the field is missing or the value is not numeric, returns `false`.
    /// When no bounds are configured, returns `false` (no constraint, no match).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::filter::FilterEvaluator;
    /// use pelisearch_core::query::RangeQuery;
    ///
    /// let filter = RangeQuery::new("price").with_lte(1000.0);
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("price".to_string(), serde_json::json!(799));
    /// let doc = Document::new("doc1", fields).unwrap();
    ///
    /// assert!(filter.evaluate(&doc));
    /// ```
    fn evaluate(&self, doc: &Document) -> bool {
        let value = match doc.get_field(&self.field) {
            Some(serde_json::Value::Number(n)) => match n.as_f64() {
                Some(v) => v,
                None => return false,
            },
            _ => return false,
        };

        if let Some(bound) = self.gt {
            if !(value > bound) {
                return false;
            }
        }
        if let Some(bound) = self.gte {
            if !(value >= bound) {
                return false;
            }
        }
        if let Some(bound) = self.lt {
            if !(value < bound) {
                return false;
            }
        }
        if let Some(bound) = self.lte {
            if !(value <= bound) {
                return false;
            }
        }

        // At least one bound must be set to match
        self.gt.is_some() || self.gte.is_some() || self.lt.is_some() || self.lte.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::filter::FilterEvaluator;
    use crate::query::RangeQuery;

    fn make_doc(fields: Vec<(&str, serde_json::Value)>) -> Document {
        let mut map = HashMap::new();
        for (k, v) in fields {
            map.insert(k.to_string(), v);
        }
        Document::new("doc", map).unwrap()
    }

    #[test]
    fn lte_matches() {
        let filter = RangeQuery::new("price").with_lte(1000.0);
        let doc = make_doc(vec![("price", serde_json::json!(799))]);
        assert!(filter.evaluate(&doc));
    }

    #[test]
    fn lte_exact_boundary() {
        let filter = RangeQuery::new("price").with_lte(100.0);
        let doc = make_doc(vec![("price", serde_json::json!(100))]);
        assert!(filter.evaluate(&doc));
    }

    #[test]
    fn lte_excludes_above() {
        let filter = RangeQuery::new("price").with_lte(100.0);
        let doc = make_doc(vec![("price", serde_json::json!(101))]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn gte_matches() {
        let filter = RangeQuery::new("price").with_gte(50.0);
        let doc = make_doc(vec![("price", serde_json::json!(100))]);
        assert!(filter.evaluate(&doc));
    }

    #[test]
    fn gte_exact_boundary() {
        let filter = RangeQuery::new("price").with_gte(50.0);
        let doc = make_doc(vec![("price", serde_json::json!(50))]);
        assert!(filter.evaluate(&doc));
    }

    #[test]
    fn gte_excludes_below() {
        let filter = RangeQuery::new("price").with_gte(50.0);
        let doc = make_doc(vec![("price", serde_json::json!(49))]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn gt_matches() {
        let filter = RangeQuery::new("age").with_gt(18.0);
        let doc = make_doc(vec![("age", serde_json::json!(19))]);
        assert!(filter.evaluate(&doc));
    }

    #[test]
    fn gt_excludes_boundary() {
        let filter = RangeQuery::new("age").with_gt(18.0);
        let doc = make_doc(vec![("age", serde_json::json!(18))]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn lt_matches() {
        let filter = RangeQuery::new("age").with_lt(65.0);
        let doc = make_doc(vec![("age", serde_json::json!(30))]);
        assert!(filter.evaluate(&doc));
    }

    #[test]
    fn lt_excludes_boundary() {
        let filter = RangeQuery::new("age").with_lt(65.0);
        let doc = make_doc(vec![("age", serde_json::json!(65))]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn combined_gte_and_lte() {
        let filter = RangeQuery::new("price").with_gte(10.0).with_lte(100.0);
        assert!(filter.evaluate(&make_doc(vec![("price", serde_json::json!(50))])));
        assert!(filter.evaluate(&make_doc(vec![("price", serde_json::json!(10))])));
        assert!(filter.evaluate(&make_doc(vec![("price", serde_json::json!(100))])));
        assert!(!filter.evaluate(&make_doc(vec![("price", serde_json::json!(5))])));
        assert!(!filter.evaluate(&make_doc(vec![("price", serde_json::json!(101))])));
    }

    #[test]
    fn missing_field_returns_false() {
        let filter = RangeQuery::new("price").with_lte(100.0);
        let doc = make_doc(vec![("title", serde_json::json!("hello"))]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn non_numeric_field_returns_false() {
        let filter = RangeQuery::new("price").with_lte(100.0);
        let doc = make_doc(vec![("price", serde_json::json!("not_a_number"))]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn null_field_returns_false() {
        let filter = RangeQuery::new("field").with_lte(100.0);
        let doc = make_doc(vec![("field", serde_json::Value::Null)]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn no_bounds_returns_false() {
        let filter = RangeQuery::new("price");
        let doc = make_doc(vec![("price", serde_json::json!(50))]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn integer_values_work() {
        let filter = RangeQuery::new("stock").with_gte(0.0).with_lte(100.0);
        let doc = make_doc(vec![("stock", serde_json::json!(42))]);
        assert!(filter.evaluate(&doc));
    }

    #[test]
    fn negative_values_work() {
        let filter = RangeQuery::new("balance").with_gte(-100.0).with_lte(0.0);
        let doc = make_doc(vec![("balance", serde_json::json!(-50))]);
        assert!(filter.evaluate(&doc));
        let doc2 = make_doc(vec![("balance", serde_json::json!(-101))]);
        assert!(!filter.evaluate(&doc2));
    }

    #[test]
    fn float_values_work() {
        let filter = RangeQuery::new("rating").with_gte(3.5).with_lte(4.5);
        let doc = make_doc(vec![("rating", serde_json::json!(4.2))]);
        assert!(filter.evaluate(&doc));
        let doc2 = make_doc(vec![("rating", serde_json::json!(3.4))]);
        assert!(!filter.evaluate(&doc2));
    }

    #[test]
    fn boolean_field_returns_false() {
        let filter = RangeQuery::new("active").with_lte(100.0);
        let doc = make_doc(vec![("active", serde_json::json!(true))]);
        assert!(!filter.evaluate(&doc));
    }
}
