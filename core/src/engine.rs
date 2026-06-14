use std::collections::HashMap;

use crate::document::Document;
use crate::error::SearchError;
use crate::index::InvertedIndex;
use crate::search;
use crate::types::SearchResult;

/// High-level search engine that coordinates tokenization, indexing, and search.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::document::Document;
/// use pelisearch_core::engine::SearchEngine;
///
/// let mut engine = SearchEngine::new();
///
/// let mut fields = HashMap::new();
/// fields.insert("title".to_string(), serde_json::json!("electric bike"));
/// let doc = Document::new("doc1", fields).unwrap();
/// engine.add_document(doc).unwrap();
///
/// let results = engine.search("bike").unwrap();
/// assert_eq!(results.len(), 1);
/// assert_eq!(results[0].document_id, "doc1");
/// ```
pub struct SearchEngine {
    documents: HashMap<String, Document>,
    index: InvertedIndex,
}

impl SearchEngine {
    /// Create a new empty `SearchEngine`.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            index: InvertedIndex::new(),
        }
    }

    /// Add a document to the engine.
    ///
    /// The document's string field values are tokenized and indexed for search.
    /// Returns an error if the document has an empty ID or a document with the
    /// same ID already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let mut engine = SearchEngine::new();
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("hello world"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// engine.add_document(doc).unwrap();
    /// ```
    pub fn add_document(&mut self, document: Document) -> Result<(), SearchError> {
        if document.id.is_empty() {
            return Err(SearchError::InvalidDocumentId(
                "document ID must not be empty".to_string(),
            ));
        }

        if self.documents.contains_key(&document.id) {
            return Err(SearchError::DocumentAlreadyExists(document.id.clone()));
        }

        let text = self.extract_text(&document);
        self.index.add_document(&document.id, &text)?;
        self.documents.insert(document.id.clone(), document);
        Ok(())
    }

    /// Remove a document from the engine by its ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let mut engine = SearchEngine::new();
    /// let doc = Document::new("doc1", HashMap::new()).unwrap();
    /// engine.add_document(doc).unwrap();
    /// engine.remove_document("doc1").unwrap();
    /// assert!(engine.get_document("doc1").is_err());
    /// ```
    pub fn remove_document(&mut self, id: &str) -> Result<(), SearchError> {
        if !self.documents.contains_key(id) {
            return Err(SearchError::DocumentNotFound(id.to_string()));
        }
        self.index.remove_document(id);
        self.documents.remove(id);
        Ok(())
    }

    /// Search for documents matching the query.
    ///
    /// Returns a list of `SearchResult` entries sorted by relevance
    /// (number of matching terms).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let mut engine = SearchEngine::new();
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("electric bike"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// engine.add_document(doc).unwrap();
    ///
    /// let results = engine.search("electric bike").unwrap();
    /// assert_eq!(results.len(), 1);
    /// assert_eq!(results[0].score, 2.0);
    /// ```
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, SearchError> {
        Ok(search::search(&self.index, query))
    }

    /// Get a document by its ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let mut engine = SearchEngine::new();
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("hello"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// engine.add_document(doc).unwrap();
    ///
    /// let retrieved = engine.get_document("doc1").unwrap();
    /// assert_eq!(retrieved.id, "doc1");
    /// ```
    pub fn get_document(&self, id: &str) -> Result<&Document, SearchError> {
        self.documents
            .get(id)
            .ok_or_else(|| SearchError::DocumentNotFound(id.to_string()))
    }

    fn extract_text(&self, document: &Document) -> String {
        let mut parts: Vec<String> = Vec::new();
        for value in document.fields.values() {
            match value {
                serde_json::Value::String(s) => parts.push(s.clone()),
                serde_json::Value::Number(n) => parts.push(n.to_string()),
                serde_json::Value::Bool(b) => parts.push(b.to_string()),
                serde_json::Value::Array(arr) => {
                    for v in arr {
                        if let serde_json::Value::String(s) = v {
                            parts.push(s.clone());
                        }
                    }
                }
                _ => {}
            }
        }
        parts.join(" ")
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
    use std::collections::HashMap;

    #[test]
    fn add_and_search_documents() {
        let mut engine = SearchEngine::new();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("electric bike"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document(doc).unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("walking shoes"));
        let doc = Document::new("doc2", fields).unwrap();
        engine.add_document(doc).unwrap();

        let results = engine.search("bike").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc1");
    }

    #[test]
    fn remove_document() {
        let mut engine = SearchEngine::new();

        let doc = Document::new("doc1", HashMap::new()).unwrap();
        engine.add_document(doc).unwrap();

        engine.remove_document("doc1").unwrap();
        assert!(engine.get_document("doc1").is_err());
    }

    #[test]
    fn add_duplicate_document_returns_error() {
        let mut engine = SearchEngine::new();

        let doc = Document::new("doc1", HashMap::new()).unwrap();
        engine.add_document(doc).unwrap();

        let doc2 = Document::new("doc1", HashMap::new()).unwrap();
        let err = engine.add_document(doc2).unwrap_err();
        assert!(matches!(err, SearchError::DocumentAlreadyExists(_)));
    }

    #[test]
    fn remove_nonexistent_document_returns_error() {
        let mut engine = SearchEngine::new();
        let err = engine.remove_document("nonexistent").unwrap_err();
        assert!(matches!(err, SearchError::DocumentNotFound(_)));
    }

    #[test]
    fn search_after_remove() {
        let mut engine = SearchEngine::new();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("hello world"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document(doc).unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("hello there"));
        let doc = Document::new("doc2", fields).unwrap();
        engine.add_document(doc).unwrap();

        engine.remove_document("doc1").unwrap();
        let results = engine.search("hello").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc2");
    }

    #[test]
    fn get_document_returns_document() {
        let mut engine = SearchEngine::new();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("test"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document(doc).unwrap();

        let retrieved = engine.get_document("doc1").unwrap();
        assert_eq!(retrieved.id, "doc1");
        assert_eq!(
            retrieved.get_field("title"),
            Some(&serde_json::json!("test"))
        );
    }

    #[test]
    fn empty_search_returns_empty() {
        let engine = SearchEngine::new();
        let results = engine.search("").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn index_multiple_fields() {
        let mut engine = SearchEngine::new();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("electric bike"));
        fields.insert("description".to_string(), serde_json::json!("fast commuter"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document(doc).unwrap();

        let results = engine.search("commuter").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc1");
    }
}
