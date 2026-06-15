use crate::document::Document;
use crate::filter::evaluator::FilterEvaluator;
use crate::query::TermQuery;

impl FilterEvaluator for TermQuery {
    /// Returns `true` when the document has the specified field and its value
    /// is a JSON string that exactly matches `self.value`.
    ///
    /// Missing fields and non-string values return `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::filter::FilterEvaluator;
    /// use pelisearch_core::query::TermQuery;
    ///
    /// let filter = TermQuery::new("category", "electronics");
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("category".to_string(), serde_json::json!("electronics"));
    /// let doc = Document::new("doc1", fields).unwrap();
    ///
    /// assert!(filter.evaluate(&doc));
    /// ```
    fn evaluate(&self, doc: &Document) -> bool {
        match doc.get_field(&self.field) {
            Some(serde_json::Value::String(s)) => s == &self.value,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::filter::FilterEvaluator;
    use crate::query::TermQuery;

    fn make_doc(fields: Vec<(&str, serde_json::Value)>) -> Document {
        let mut map = HashMap::new();
        for (k, v) in fields {
            map.insert(k.to_string(), v);
        }
        Document::new("doc", map).unwrap()
    }

    #[test]
    fn exact_match_succeeds() {
        let filter = TermQuery::new("category", "electronics");
        let doc = make_doc(vec![("category", serde_json::json!("electronics"))]);
        assert!(filter.evaluate(&doc));
    }

    #[test]
    fn non_matching_value_returns_false() {
        let filter = TermQuery::new("category", "electronics");
        let doc = make_doc(vec![("category", serde_json::json!("books"))]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn missing_field_returns_false() {
        let filter = TermQuery::new("category", "electronics");
        let doc = make_doc(vec![("title", serde_json::json!("hello"))]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn non_string_value_returns_false() {
        let filter = TermQuery::new("price", "100");
        let doc = make_doc(vec![("price", serde_json::json!(100))]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn case_sensitive_comparison() {
        let filter = TermQuery::new("status", "Active");
        let doc = make_doc(vec![("status", serde_json::json!("active"))]);
        assert!(!filter.evaluate(&doc));

        let doc2 = make_doc(vec![("status", serde_json::json!("Active"))]);
        assert!(filter.evaluate(&doc2));
    }

    #[test]
    fn empty_field_name() {
        let filter = TermQuery::new("", "value");
        let doc = make_doc(vec![]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn partial_string_mismatch() {
        let filter = TermQuery::new("category", "electronics");
        let doc = make_doc(vec![("category", serde_json::json!("electronic"))]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn multiple_fields_ignores_others() {
        let filter = TermQuery::new("color", "red");
        let doc = make_doc(vec![
            ("color", serde_json::json!("red")),
            ("size", serde_json::json!("large")),
        ]);
        assert!(filter.evaluate(&doc));
    }

    #[test]
    fn null_value_returns_false() {
        let filter = TermQuery::new("field", "anything");
        let doc = make_doc(vec![("field", serde_json::Value::Null)]);
        assert!(!filter.evaluate(&doc));
    }

    #[test]
    fn array_value_returns_false() {
        let filter = TermQuery::new("tags", "rust");
        let doc = make_doc(vec![("tags", serde_json::json!(["rust", "cargo"]))]);
        assert!(!filter.evaluate(&doc));
    }
}
