use crate::document::Document;
use crate::error::SearchError;
use crate::index::IndexManager;
use crate::schema::Mapping;
use crate::types::{SearchHit, SearchResponse};

/// High-level coordinator that routes operations to named indexes.
///
/// Wraps an `IndexManager` and provides convenience methods for
/// creating indexes and operating on documents within them.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::document::Document;
/// use pelisearch_core::engine::SearchEngine;
///
/// let mut engine = SearchEngine::new();
/// engine.create_index("products").unwrap();
///
/// let mut fields = HashMap::new();
/// fields.insert("title".to_string(), serde_json::json!("electric bike"));
/// let doc = Document::new("doc1", fields).unwrap();
/// engine.add_document("products", doc).unwrap();
///
/// let results = engine.search("products", "bike").unwrap();
/// assert_eq!(results.len(), 1);
/// ```
pub struct SearchEngine {
    manager: IndexManager,
}

impl SearchEngine {
    /// Create a new empty `SearchEngine`.
    pub fn new() -> Self {
        Self {
            manager: IndexManager::new(),
        }
    }

    /// Create a new named index with an empty schema mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let mut engine = SearchEngine::new();
    /// engine.create_index("products").unwrap();
    /// assert!(engine.list_indexes().contains(&"products".to_string()));
    /// ```
    pub fn create_index(&mut self, name: impl Into<String>) -> Result<(), SearchError> {
        self.manager.create_index(name)
    }

    /// Create a new named index with the given schema mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::engine::SearchEngine;
    /// use pelisearch_core::schema::{Mapping, Field, FieldType};
    ///
    /// let mut engine = SearchEngine::new();
    /// let mapping = Mapping::new(vec![
    ///     Field::new("title", FieldType::Text, true),
    /// ]);
    /// engine.create_index_with_mapping("articles", mapping).unwrap();
    /// ```
    pub fn create_index_with_mapping(
        &mut self,
        name: impl Into<String>,
        mapping: Mapping,
    ) -> Result<(), SearchError> {
        self.manager.create_index_with_mapping(name, mapping)
    }

    /// List all index names.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let mut engine = SearchEngine::new();
    /// engine.create_index("products").unwrap();
    /// engine.create_index("users").unwrap();
    /// assert_eq!(engine.list_indexes(), vec!["products", "users"]);
    /// ```
    pub fn list_indexes(&self) -> Vec<String> {
        self.manager.list_indexes()
    }

    /// Add a document to a specific index.
    ///
    /// The document is validated against the index's schema mapping,
    /// then tokenized and indexed for search.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let mut engine = SearchEngine::new();
    /// engine.create_index("products").unwrap();
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("hello world"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// engine.add_document("products", doc).unwrap();
    /// ```
    pub fn add_document(
        &mut self,
        index_name: &str,
        document: Document,
    ) -> Result<(), SearchError> {
        self.manager.add_document(index_name, document)
    }

    /// Remove a document from a specific index.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let mut engine = SearchEngine::new();
    /// engine.create_index("products").unwrap();
    ///
    /// let doc = Document::new("doc1", HashMap::new()).unwrap();
    /// engine.add_document("products", doc).unwrap();
    /// engine.remove_document("products", "doc1").unwrap();
    /// ```
    pub fn remove_document(
        &mut self,
        index_name: &str,
        doc_id: &str,
    ) -> Result<(), SearchError> {
        self.manager.remove_document(index_name, doc_id)
    }

    /// Search a specific index for documents matching the query using BM25 ranking.
    ///
    /// Returns results sorted by relevance descending.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let mut engine = SearchEngine::new();
    /// engine.create_index("products").unwrap();
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("electric bike"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// engine.add_document("products", doc).unwrap();
    ///
    /// let results = engine.search("products", "electric bike").unwrap();
    /// assert_eq!(results.len(), 1);
    /// ```
    pub fn search(
        &self,
        index_name: &str,
        query: &str,
    ) -> Result<Vec<SearchHit>, SearchError> {
        self.manager.search(index_name, query)
    }

    /// Search a specific index with BM25 ranking and return per-document
    /// score explanations.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let mut engine = SearchEngine::new();
    /// engine.create_index("products").unwrap();
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("electric bike"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// engine.add_document("products", doc).unwrap();
    ///
    /// let response = engine.search_with_explanations("products", "electric").unwrap();
    /// assert_eq!(response.results.len(), 1);
    /// assert_eq!(response.explanations.len(), 1);
    /// ```
    pub fn search_with_explanations(
        &self,
        index_name: &str,
        query: &str,
    ) -> Result<SearchResponse, SearchError> {
        let index = self.manager.get_index(index_name)?;
        let results = index.search(query);

        let explanations: Vec<(String, Vec<crate::ranking::explanation::ScoreExplanation>)> =
            results
                .iter()
                .map(|r| {
                    let exps = crate::ranking::explanation::explain_document(
                        query,
                        &r.document_id,
                        &index.stats_ref(),
                    );
                    (r.document_id.clone(), exps)
                })
                .collect();

        Ok(SearchResponse::new(results, explanations))
    }

    /// Get a document from a specific index.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let mut engine = SearchEngine::new();
    /// engine.create_index("products").unwrap();
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("hello"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// engine.add_document("products", doc).unwrap();
    ///
    /// let retrieved = engine.get_document("products", "doc1").unwrap();
    /// assert_eq!(retrieved.id, "doc1");
    /// ```
    pub fn get_document(&self, index_name: &str, doc_id: &str) -> Result<&Document, SearchError> {
        let index = self.manager.get_index(index_name)?;
        index.get_document(doc_id)
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Field, FieldType};
    use std::collections::HashMap;

    #[test]
    fn create_index_and_list() {
        let mut engine = SearchEngine::new();
        engine.create_index("products").unwrap();
        engine.create_index("users").unwrap();
        assert_eq!(engine.list_indexes(), vec!["products", "users"]);
    }

    #[test]
    fn create_duplicate_index_fails() {
        let mut engine = SearchEngine::new();
        engine.create_index("test").unwrap();
        assert!(engine.create_index("test").is_err());
    }

    #[test]
    fn add_and_search_documents() {
        let mut engine = SearchEngine::new();
        engine.create_index("products").unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("electric bike"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document("products", doc).unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("walking shoes"));
        let doc = Document::new("doc2", fields).unwrap();
        engine.add_document("products", doc).unwrap();

        let results = engine.search("products", "bike").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc1");
    }

    #[test]
    fn search_in_nonexistent_index_fails() {
        let engine = SearchEngine::new();
        let err = engine.search("nonexistent", "hello").unwrap_err();
        assert!(matches!(err, SearchError::Internal(_)));
    }

    #[test]
    fn remove_document() {
        let mut engine = SearchEngine::new();
        engine.create_index("test").unwrap();

        let doc = Document::new("doc1", HashMap::new()).unwrap();
        engine.add_document("test", doc).unwrap();
        engine.remove_document("test", "doc1").unwrap();
        assert!(engine.get_document("test", "doc1").is_err());
    }

    #[test]
    fn add_document_with_schema() {
        let mut engine = SearchEngine::new();
        let mapping = Mapping::new(vec![Field::new("title", FieldType::Text, true)]);
        engine.create_index_with_mapping("articles", mapping).unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("hello world"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document("articles", doc).unwrap();

        let results = engine.search("articles", "hello").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn schema_validation_rejects_bad_document() {
        let mut engine = SearchEngine::new();
        let mapping = Mapping::new(vec![
            Field::new("title", FieldType::Text, true),
            Field::new("price", FieldType::Float, true),
        ]);
        engine.create_index_with_mapping("products", mapping).unwrap();

        // Missing required field "price"
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("widget"));
        let doc = Document::new("doc1", fields).unwrap();
        let err = engine.add_document("products", doc).unwrap_err();
        assert!(matches!(err, SearchError::SchemaValidationError(_)));
    }

    #[test]
    fn search_after_remove() {
        let mut engine = SearchEngine::new();
        engine.create_index("test").unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("hello world"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document("test", doc).unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("hello there"));
        let doc = Document::new("doc2", fields).unwrap();
        engine.add_document("test", doc).unwrap();

        engine.remove_document("test", "doc1").unwrap();
        let results = engine.search("test", "hello").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc2");
    }

    #[test]
    fn get_document_returns_document() {
        let mut engine = SearchEngine::new();
        engine.create_index("test").unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("test"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document("test", doc).unwrap();

        let retrieved = engine.get_document("test", "doc1").unwrap();
        assert_eq!(retrieved.id, "doc1");
    }

    #[test]
    fn empty_search_returns_empty() {
        let mut engine = SearchEngine::new();
        engine.create_index("test").unwrap();
        let results = engine.search("test", "").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn index_multiple_fields() {
        let mut engine = SearchEngine::new();
        engine.create_index("test").unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("electric bike"));
        fields.insert("description".to_string(), serde_json::json!("fast commuter"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document("test", doc).unwrap();

        let results = engine.search("test", "commuter").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_with_explanations_engine() {
        let mut engine = SearchEngine::new();
        engine.create_index("test").unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("electric bike"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document("test", doc).unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("electric car"));
        let doc = Document::new("doc2", fields).unwrap();
        engine.add_document("test", doc).unwrap();

        let response = engine.search_with_explanations("test", "electric bike").unwrap();
        assert_eq!(response.results.len(), 2);
        assert_eq!(response.explanations.len(), 2);

        for (doc_id, exps) in &response.explanations {
            assert!(!exps.is_empty());
            assert!(response.results.iter().any(|r| &r.document_id == doc_id));
        }
    }

    #[test]
    fn cross_index_isolation() {
        let mut engine = SearchEngine::new();
        engine.create_index("products").unwrap();
        engine.create_index("users").unwrap();

        let mut fields = HashMap::new();
        fields.insert("name".to_string(), serde_json::json!("widget"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document("products", doc).unwrap();

        let mut fields = HashMap::new();
        fields.insert("name".to_string(), serde_json::json!("Alice"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document("users", doc).unwrap();

        // Same doc ID in different indexes — no conflict
        assert_eq!(
            engine.get_document("products", "doc1").unwrap().id,
            "doc1"
        );
        assert_eq!(engine.get_document("users", "doc1").unwrap().id, "doc1");
    }
}
