use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::SearchError;

/// A document containing an ID and arbitrary JSON fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    /// Unique identifier for this document.
    pub id: String,
    /// Arbitrary field-value pairs.
    pub fields: HashMap<String, serde_json::Value>,
}

impl Document {
    /// Create a new `Document`.
    ///
    /// Returns `Err(SearchError::InvalidDocumentId)` if the ID is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::document::Document;
    /// use std::collections::HashMap;
    ///
    /// let doc = Document::new("doc1", HashMap::new()).unwrap();
    /// assert_eq!(doc.id, "doc1");
    /// ```
    pub fn new(
        id: impl Into<String>,
        fields: HashMap<String, serde_json::Value>,
    ) -> Result<Self, SearchError> {
        let id = id.into().trim().to_string();
        if id.is_empty() {
            return Err(SearchError::InvalidDocumentId(
                "document ID must not be empty".to_string(),
            ));
        }
        if id.len() > 256 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
            return Err(SearchError::InvalidDocumentId(
                "document ID must be 1-256 characters and contain only alphanumeric characters, dashes, underscores, and dots".to_string(),
            ));
        }
        Ok(Self { id, fields })
    }

    /// Retrieve a field value by name.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::document::Document;
    /// use std::collections::HashMap;
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("hello"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// assert_eq!(doc.get_field("title"), Some(&serde_json::json!("hello")));
    /// assert_eq!(doc.get_field("missing"), None);
    /// ```
    pub fn get_field(&self, name: &str) -> Option<&serde_json::Value> {
        self.fields.get(name)
    }

    /// Add or update a field on this document.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::document::Document;
    /// use std::collections::HashMap;
    ///
    /// let mut doc = Document::new("doc1", HashMap::new()).unwrap();
    /// doc.set_field("title".to_string(), serde_json::json!("hello"));
    /// assert_eq!(doc.get_field("title"), Some(&serde_json::json!("hello")));
    /// ```
    pub fn set_field(&mut self, name: String, value: serde_json::Value) {
        self.fields.insert(name, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_document_with_valid_id() {
        let doc = Document::new("doc1", HashMap::new()).unwrap();
        assert_eq!(doc.id, "doc1");
        assert!(doc.fields.is_empty());
    }

    #[test]
    fn create_document_with_empty_id_returns_error() {
        let err = Document::new("", HashMap::new()).unwrap_err();
        assert!(matches!(err, SearchError::InvalidDocumentId(_)));
    }

    #[test]
    fn add_and_retrieve_field() {
        let mut doc = Document::new("doc1", HashMap::new()).unwrap();
        doc.set_field("title".to_string(), serde_json::json!("hello world"));
        assert_eq!(
            doc.get_field("title"),
            Some(&serde_json::json!("hello world"))
        );
    }

    #[test]
    fn retrieve_missing_field_returns_none() {
        let doc = Document::new("doc1", HashMap::new()).unwrap();
        assert_eq!(doc.get_field("nonexistent"), None);
    }

    #[test]
    fn document_with_fields_constructor() {
        let mut fields = HashMap::new();
        fields.insert("age".to_string(), serde_json::json!(30));
        fields.insert("name".to_string(), serde_json::json!("Alice"));
        let doc = Document::new("person_1", fields).unwrap();
        assert_eq!(doc.get_field("age"), Some(&serde_json::json!(30)));
        assert_eq!(doc.get_field("name"), Some(&serde_json::json!("Alice")));
    }

    #[test]
    fn overwrite_field() {
        let mut doc = Document::new("doc1", HashMap::new()).unwrap();
        doc.set_field("x".to_string(), serde_json::json!(1));
        doc.set_field("x".to_string(), serde_json::json!(2));
        assert_eq!(doc.get_field("x"), Some(&serde_json::json!(2)));
    }

    #[test]
    fn serde_roundtrip() {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("test"));
        let doc = Document::new("doc1", fields).unwrap();
        let json = serde_json::to_string(&doc).unwrap();
        let restored: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, restored);
    }
}
