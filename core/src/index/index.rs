use std::collections::HashMap;

use crate::document::Document;
use crate::error::SearchError;
use crate::index::inverted::InvertedIndex;
use crate::ranking::statistics::CollectionStats;
use crate::schema::Mapping;
use crate::search;
use crate::types::SearchResult;

/// A named index that owns documents, an inverted index, statistics, and a schema mapping.
///
/// Each `Index` instance represents one logical index (e.g. "products", "users")
/// with its own document store, search index, and schema.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::document::Document;
/// use pelisearch_core::index::Index;
/// use pelisearch_core::schema::{Mapping, Field, FieldType};
///
/// let mapping = Mapping::new(vec![
///     Field::new("title", FieldType::Text, true),
/// ]);
/// let mut index = Index::new("products", mapping);
///
/// let mut fields = HashMap::new();
/// fields.insert("title".to_string(), serde_json::json!("electric bike"));
/// let doc = Document::new("doc1", fields).unwrap();
/// index.add_document(doc).unwrap();
///
/// let results = index.search("bike");
/// assert_eq!(results.len(), 1);
/// ```
pub struct Index {
    name: String,
    documents: HashMap<String, Document>,
    inverted_index: InvertedIndex,
    stats: CollectionStats,
    mapping: Mapping,
}

impl Index {
    /// Create a new `Index` with the given name and mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::Index;
    /// use pelisearch_core::schema::Mapping;
    ///
    /// let index = Index::new("products", Mapping::new(vec![]));
    /// assert_eq!(index.name(), "products");
    /// ```
    pub fn new(name: impl Into<String>, mapping: Mapping) -> Self {
        Self {
            name: name.into(),
            documents: HashMap::new(),
            inverted_index: InvertedIndex::new(),
            stats: CollectionStats::new(),
            mapping,
        }
    }

    /// The name of this index.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Add a document to this index.
    ///
    /// The document is validated against the index's schema mapping,
    /// then tokenized and indexed for search.
    ///
    /// Returns an error if the document ID is empty, already exists,
    /// or fails schema validation.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::index::Index;
    /// use pelisearch_core::schema::{Mapping, Field, FieldType};
    ///
    /// let mapping = Mapping::new(vec![
    ///     Field::new("title", FieldType::Text, true),
    /// ]);
    /// let mut index = Index::new("articles", mapping);
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("hello world"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// index.add_document(doc).unwrap();
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

        self.mapping.validate_document(&document)?;

        let text = self.extract_text(&document);
        self.inverted_index.add_document(&document.id, &text)?;
        self.stats.update_document(&document.id, &text);
        self.documents.insert(document.id.clone(), document);
        Ok(())
    }

    /// Remove a document from this index by its ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::index::Index;
    /// use pelisearch_core::schema::Mapping;
    ///
    /// let mut index = Index::new("test", Mapping::new(vec![]));
    /// let doc = Document::new("doc1", HashMap::new()).unwrap();
    /// index.add_document(doc).unwrap();
    /// index.remove_document("doc1").unwrap();
    /// assert!(index.get_document("doc1").is_err());
    /// ```
    pub fn remove_document(&mut self, id: &str) -> Result<(), SearchError> {
        if !self.documents.contains_key(id) {
            return Err(SearchError::DocumentNotFound(id.to_string()));
        }
        self.inverted_index.remove_document(id);
        self.stats.remove_document(id);
        self.documents.remove(id);
        Ok(())
    }

    /// Search this index for documents matching the query using BM25 ranking.
    ///
    /// Returns results sorted by relevance descending.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::index::Index;
    /// use pelisearch_core::schema::{Mapping, Field, FieldType};
    ///
    /// let mapping = Mapping::new(vec![
    ///     Field::new("title", FieldType::Text, true),
    /// ]);
    /// let mut index = Index::new("articles", mapping);
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("electric bike"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// index.add_document(doc).unwrap();
    ///
    /// let results = index.search("bike");
    /// assert_eq!(results.len(), 1);
    /// ```
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        search::search(&self.inverted_index, &self.stats, query)
    }

    /// Search with OR semantics — returns documents containing ANY query term.
    ///
    /// Unlike `search()` which uses AND semantics for multi-term queries,
    /// this returns all documents that match at least one term.
    /// Useful for query pipelines that need broad candidate collection
    /// (e.g., before applying field filters).
    pub fn search_any(&self, query: &str) -> Vec<SearchResult> {
        search::search_any(&self.inverted_index, &self.stats, query)
    }

    /// Get a document by its ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::index::Index;
    /// use pelisearch_core::schema::Mapping;
    ///
    /// let mut index = Index::new("test", Mapping::new(vec![]));
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("hello"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// index.add_document(doc).unwrap();
    ///
    /// let retrieved = index.get_document("doc1").unwrap();
    /// assert_eq!(retrieved.id, "doc1");
    /// ```
    pub fn get_document(&self, id: &str) -> Result<&Document, SearchError> {
        self.documents
            .get(id)
            .ok_or_else(|| SearchError::DocumentNotFound(id.to_string()))
    }

    /// The schema mapping for this index.
    pub fn mapping(&self) -> &Mapping {
        &self.mapping
    }

    /// Reference to the collection statistics (used by search with explanations).
    pub fn stats_ref(&self) -> &CollectionStats {
        &self.stats
    }

    /// Clone the inverted index (used by snapshots).
    pub fn inverted_index_clone(&self) -> InvertedIndex {
        self.inverted_index.clone()
    }

    /// Return all document IDs currently stored in this index.
    pub fn list_document_ids(&self) -> Vec<String> {
        self.documents.keys().cloned().collect()
    }

    /// Returns the number of documents in the index.
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Recompute collection statistics from all stored documents.
    ///
    /// This ensures consistency after loading state from disk (segments or
    /// snapshots) where documents may have been deserialized without
    /// re-running the statistics pipeline.
    pub fn rebuild_stats(&mut self) {
        let mut new_stats = CollectionStats::new();
        for doc in self.documents.values() {
            let text = self.extract_text(doc);
            new_stats.update_document(&doc.id, &text);
        }
        self.stats = new_stats;
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

impl std::fmt::Debug for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Index")
            .field("name", &self.name)
            .field("document_count", &self.documents.len())
            .field("mapping", &self.mapping)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Field, FieldType};

    fn empty_mapping() -> Mapping {
        Mapping::new(vec![])
    }

    fn text_mapping() -> Mapping {
        Mapping::new(vec![Field::new("title", FieldType::Text, true)])
    }

    fn make_doc(id: &str, title: &str) -> Document {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!(title));
        Document::new(id, fields).unwrap()
    }

    #[test]
    fn add_and_search_documents() {
        let mut index = Index::new("test", text_mapping());
        index.add_document(make_doc("doc1", "electric bike")).unwrap();
        index.add_document(make_doc("doc2", "walking shoes")).unwrap();

        let results = index.search("bike");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc1");
    }

    #[test]
    fn remove_document() {
        let mut index = Index::new("test", empty_mapping());
        let doc = Document::new("doc1", HashMap::new()).unwrap();
        index.add_document(doc).unwrap();
        index.remove_document("doc1").unwrap();
        assert!(index.get_document("doc1").is_err());
    }

    #[test]
    fn add_duplicate_document_returns_error() {
        let mut index = Index::new("test", empty_mapping());
        let doc = Document::new("doc1", HashMap::new()).unwrap();
        index.add_document(doc).unwrap();

        let doc2 = Document::new("doc1", HashMap::new()).unwrap();
        let err = index.add_document(doc2).unwrap_err();
        assert!(matches!(err, SearchError::DocumentAlreadyExists(_)));
    }

    #[test]
    fn remove_nonexistent_document_returns_error() {
        let mut index = Index::new("test", empty_mapping());
        let err = index.remove_document("nonexistent").unwrap_err();
        assert!(matches!(err, SearchError::DocumentNotFound(_)));
    }

    #[test]
    fn schema_validation_on_add() {
        let mapping = Mapping::new(vec![
            Field::new("title", FieldType::Text, true),
            Field::new("price", FieldType::Float, true),
        ]);
        let mut index = Index::new("products", mapping);

        // Missing required field 'price'
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("widget"));
        let doc = Document::new("doc1", fields).unwrap();
        let err = index.add_document(doc).unwrap_err();
        assert!(matches!(err, SearchError::SchemaValidationError(_)));
    }

    #[test]
    fn search_after_remove() {
        let mut index = Index::new("test", text_mapping());
        index.add_document(make_doc("doc1", "hello world")).unwrap();
        index.add_document(make_doc("doc2", "hello there")).unwrap();

        index.remove_document("doc1").unwrap();
        let results = index.search("hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc2");
    }

    #[test]
    fn get_document_returns_document() {
        let mut index = Index::new("test", text_mapping());
        index.add_document(make_doc("doc1", "test")).unwrap();

        let retrieved = index.get_document("doc1").unwrap();
        assert_eq!(retrieved.id, "doc1");
    }

    #[test]
    fn empty_search_returns_empty() {
        let index = Index::new("test", empty_mapping());
        let results = index.search("");
        assert!(results.is_empty());
    }

    #[test]
    fn index_name() {
        let index = Index::new("products", empty_mapping());
        assert_eq!(index.name(), "products");
    }

    #[test]
    fn mapping_accessor() {
        let mapping = text_mapping();
        let index = Index::new("test", mapping);
        assert!(index.mapping().field_exists("title"));
    }

    #[test]
    fn reject_empty_id() {
        let mut index = Index::new("test", empty_mapping());
        let doc = Document {
            id: String::new(),
            fields: HashMap::new(),
        };
        let err = index.add_document(doc).unwrap_err();
        assert!(matches!(err, SearchError::InvalidDocumentId(_)));
    }

    #[test]
    fn index_with_integer_field_type() {
        let mapping = Mapping::new(vec![
            Field::new("count", FieldType::Integer, true),
        ]);
        let mut index = Index::new("test", mapping);

        let mut fields = HashMap::new();
        fields.insert("count".to_string(), serde_json::json!(42));
        let doc = Document::new("doc1", fields).unwrap();
        index.add_document(doc).unwrap();

        let results = index.search("42");
        assert_eq!(results.len(), 1);
    }
}
