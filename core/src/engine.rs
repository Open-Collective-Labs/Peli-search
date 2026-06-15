use serde::Serialize;

use crate::document::Document;
use crate::error::SearchError;
use crate::index::IndexManager;
use crate::query::executor::QueryExecutor;
use crate::query::SearchRequest;
use crate::schema::Mapping;
use crate::storage::Storage;
use crate::types::{AggregationResults, SearchHit, SearchResponse};

/// Summary metadata for an index, returned by the Get Index API.
#[derive(Debug, Clone, Serialize)]
pub struct IndexInfo {
    /// Index name.
    pub name: String,
    /// Number of documents in the index.
    pub document_count: usize,
    /// Schema field definitions.
    pub fields: Vec<FieldInfo>,
}

/// A simplified view of a schema field for API responses.
#[derive(Debug, Clone, Serialize)]
pub struct FieldInfo {
    pub name: String,
    pub field_type: String,
    pub required: bool,
}

/// High-level coordinator that routes operations to named indexes.
///
/// Operates in one of two modes:
/// - **In-memory** (`new()`) — no persistence, indexes live in memory only.
/// - **Persistent** (`open()`) — uses WAL + snapshots for crash recovery.
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
    storage: Option<Storage>,
}

impl SearchEngine {
    /// Create a new in-memory `SearchEngine` with no persistence.
    pub fn new() -> Self {
        Self {
            manager: IndexManager::new(),
            storage: None,
        }
    }

    /// Open or create a persistent search engine at the given data directory.
    ///
    /// On open, the engine:
    /// - Reads the manifest to discover known indexes
    /// - Loads the latest segment or snapshot for each index
    /// - Replays any WAL entries not yet reflected on disk
    /// - Rebuilds collection statistics for consistency
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let engine = SearchEngine::open("./data").unwrap();
    /// ```
    pub fn open(path: impl Into<std::path::PathBuf>) -> Result<Self, SearchError> {
        let storage = Storage::open(path).map_err(|e| {
            SearchError::Internal(format!("failed to open storage: {e}"))
        })?;

        let mut manager = IndexManager::new();
        for name in storage.list_indexes() {
            let index = storage.get_index(&name).map_err(|e| {
                SearchError::Internal(format!("failed to load index '{name}': {e}"))
            })?;
            let mapping = index.mapping().clone();
            manager
                .create_index_with_mapping(name.clone(), mapping)
                .map_err(|e| {
                    SearchError::Internal(format!("failed to register index '{name}': {e}"))
                })?;

            // Copy documents from storage index into manager index
            for doc_id in index.list_document_ids() {
                let doc = index.get_document(&doc_id).map_err(|e| {
                    SearchError::Internal(format!("failed to read document '{doc_id}': {e}"))
                })?;
                manager
                    .add_document(&name, doc.clone())
                    .map_err(|e| {
                        SearchError::Internal(format!(
                            "failed to restore document '{doc_id}': {e}"
                        ))
                    })?;
            }
        }

        Ok(Self {
            manager,
            storage: Some(storage),
        })
    }

    /// Return a mutable reference to the underlying storage, if in persistent mode.
    fn storage_mut(&mut self) -> Option<&mut Storage> {
        self.storage.as_mut()
    }

    /// Create a new named index with an empty schema mapping.
    ///
    /// In persistent mode, the operation is written to the WAL before being
    /// applied in memory.
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
        let name: String = name.into();
        if let Some(storage) = self.storage_mut() {
            storage.create_index(&name, Mapping::new(vec![]))?;
            // Sync into manager
            let index = storage.get_index(&name)?;
            let mapping = index.mapping().clone();
            self.manager.create_index_with_mapping(name, mapping)?;
        } else {
            self.manager.create_index(name)?;
        }
        Ok(())
    }

    /// Create a new named index with the given schema mapping.
    ///
    /// In persistent mode, the operation is written to the WAL before being
    /// applied in memory.
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
        let name: String = name.into();
        if let Some(storage) = self.storage_mut() {
            storage.create_index(&name, mapping)?;
            let index = storage.get_index(&name)?;
            let mapping = index.mapping().clone();
            self.manager.create_index_with_mapping(name, mapping)?;
        } else {
            self.manager.create_index_with_mapping(name, mapping)?;
        }
        Ok(())
    }

    /// Delete a named index and all its data (persistent mode only).
    ///
    /// In persistent mode, the operation is written to the WAL and the
    /// on-disk index directory is removed.
    pub fn delete_index(&mut self, name: &str) -> Result<(), SearchError> {
        if let Some(storage) = self.storage_mut() {
            storage.delete_index(name)?;
        }
        self.manager.delete_index(name)
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

    /// Total number of documents across all indexes.
    pub fn total_document_count(&self) -> u64 {
        self.manager.total_document_count()
    }

    /// Add a document to a specific index.
    ///
    /// In persistent mode, the operation is written to the WAL (append +
    /// flush) before being applied in memory, ensuring crash durability.
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
        if let Some(storage) = self.storage_mut() {
            storage.add_document(index_name, document)?;
            // Keep manager in sync by replacing the index
            self.sync_index_from_storage(index_name)?;
        } else {
            self.manager.add_document(index_name, document)?;
        }
        Ok(())
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
        if let Some(storage) = self.storage_mut() {
            storage.remove_document(index_name, doc_id)?;
            self.sync_index_from_storage(index_name)?;
        } else {
            self.manager.remove_document(index_name, doc_id)?;
        }
        Ok(())
    }

    /// Flush in-memory state to disk (persistent mode only).
    ///
    /// Creates a point-in-time snapshot for each index and truncates the WAL.
    /// This is a no-op in in-memory mode.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pelisearch_core::engine::SearchEngine;
    ///
    /// let mut engine = SearchEngine::open("./data").unwrap();
    /// engine.flush().unwrap();
    /// ```
    pub fn flush(&mut self) -> Result<(), SearchError> {
        if let Some(storage) = self.storage_mut() {
            storage.flush()?;
        }
        Ok(())
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
    /// assert_eq!(response.hits.len(), 1);
    /// assert!(response.aggregations.is_empty());
    /// ```
    pub fn search_with_explanations(
        &self,
        index_name: &str,
        query: &str,
    ) -> Result<SearchResponse, SearchError> {
        let index = self.manager.get_index(index_name)?;
        let results = index.search(query);
        let hits: Vec<SearchHit> = results
            .into_iter()
            .map(|r| SearchHit::new(index_name, r.document_id, r.score))
            .collect();

        Ok(SearchResponse::new(hits, AggregationResults::new()))
    }

    /// Execute a structured search request against an index.
    ///
    /// Combines a full-text match query with optional term/range filters,
    /// runs BM25 ranking, and returns filtered results.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::engine::SearchEngine;
    /// use pelisearch_core::query::{Query, MatchQuery, RangeQuery, SearchRequest};
    ///
    /// let mut engine = SearchEngine::new();
    /// engine.create_index("products").unwrap();
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("electric bike"));
    /// fields.insert("price".to_string(), serde_json::json!(799));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// engine.add_document("products", doc).unwrap();
    ///
    /// let request = SearchRequest {
    ///     query: Query::Match(MatchQuery::new("title", "bike")),
    ///     filters: vec![
    ///         Query::Range(RangeQuery::new("price").with_lte(1000.0)),
    ///     ],
    ///     sort: vec![],
    ///     aggregations: vec![],
    /// };
    /// let results = engine.search_request("products", &request).unwrap();
    /// assert_eq!(results.len(), 1);
    /// ```
    pub fn search_request(
        &self,
        index_name: &str,
        request: &SearchRequest,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let index = self.manager.get_index(index_name)?;
        QueryExecutor::execute(index, request)
    }

    /// Execute a structured search request and return results with explanations.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::engine::SearchEngine;
    /// use pelisearch_core::query::{Query, MatchQuery, SearchRequest};
    ///
    /// let mut engine = SearchEngine::new();
    /// engine.create_index("products").unwrap();
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("hello world"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// engine.add_document("products", doc).unwrap();
    ///
    /// let request = SearchRequest {
    ///     query: Query::Match(MatchQuery::new("title", "hello")),
    ///     filters: vec![],
    ///     sort: vec![],
    ///     aggregations: vec![],
    /// };
    /// let response = engine.search_request_with_explanations("products", &request).unwrap();
    /// assert_eq!(response.hits.len(), 1);
    /// assert!(response.aggregations.is_empty());
    /// ```
    pub fn search_request_with_explanations(
        &self,
        index_name: &str,
        request: &SearchRequest,
    ) -> Result<SearchResponse, SearchError> {
        let index = self.manager.get_index(index_name)?;
        QueryExecutor::execute_with_explanations(index, request)
    }

    /// Check whether an index exists.
    pub fn index_exists(&self, name: &str) -> bool {
        self.manager.index_exists(name)
    }

    /// Get metadata about an index (name, document count, schema fields).
    pub fn get_index_info(&self, name: &str) -> Result<IndexInfo, SearchError> {
        let index = self.manager.get_index(name)?;
        let fields: Vec<FieldInfo> = index
            .mapping()
            .fields()
            .iter()
            .map(|f| FieldInfo {
                name: f.name.clone(),
                field_type: format!("{:?}", f.field_type),
                required: f.required,
            })
            .collect();
        Ok(IndexInfo {
            name: index.name().to_string(),
            document_count: index.list_document_ids().len(),
            fields,
        })
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

    /// Rebuild the manager index from Storage's internal state after a write.
    fn sync_index_from_storage(&mut self, name: &str) -> Result<(), SearchError> {
        let storage = match self.storage.as_ref() {
            Some(s) => s,
            None => return Ok(()),
        };
        let index = storage.get_index(name)?;
        let mapping = index.mapping().clone();

        // Re-create the index in manager with the same mapping
        self.manager.delete_index(name)?;
        self.manager.create_index_with_mapping(name.to_string(), mapping)?;

        // Copy documents
        for doc_id in index.list_document_ids() {
            let doc = index.get_document(&doc_id).map_err(|e| {
                SearchError::Internal(format!("failed to read document '{doc_id}': {e}"))
            })?;
            self.manager
                .add_document(name, doc.clone())
                .map_err(|e| {
                    SearchError::Internal(format!(
                        "failed to copy document '{doc_id}' into manager: {e}"
                    ))
                })?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for SearchEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchEngine")
            .field("indexes", &self.list_indexes())
            .field("persistent", &self.storage.is_some())
            .finish()
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
    use crate::query::{MatchQuery, Query, RangeQuery, SearchRequest};
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
        assert_eq!(response.hits.len(), 2);
        assert!(response.aggregations.is_empty());

        for hit in &response.hits {
            assert_eq!(hit.index, "test");
        }
    }

    #[test]
    fn search_request_with_filters() {
        let mut engine = SearchEngine::new();
        engine.create_index("products").unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("electric bike"));
        fields.insert("price".to_string(), serde_json::json!(799));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document("products", doc).unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("premium bike"));
        fields.insert("price".to_string(), serde_json::json!(1500));
        let doc = Document::new("doc2", fields).unwrap();
        engine.add_document("products", doc).unwrap();

        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![Query::Range(RangeQuery::new("price").with_lte(1000.0))],
            sort: vec![],
            aggregations: vec![],
        };
        let results = engine.search_request("products", &request).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc1");
    }

    #[test]
    fn search_request_explanations() {
        let mut engine = SearchEngine::new();
        engine.create_index("test").unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("hello world"));
        let doc = Document::new("doc1", fields).unwrap();
        engine.add_document("test", doc).unwrap();

        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "hello")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
        };
        let response = engine.search_request_with_explanations("test", &request).unwrap();
        assert_eq!(response.hits.len(), 1);
        assert!(response.aggregations.is_empty());
    }

    #[test]
    fn search_request_in_nonexistent_index_fails() {
        let engine = SearchEngine::new();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "hello")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
        };
        let err = engine.search_request("nonexistent", &request).unwrap_err();
        assert!(matches!(err, SearchError::Internal(_)));
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

    #[test]
    fn open_and_recover_persistent_engine() {
        let dir = tempfile::tempdir().unwrap();

        // Session 1: open, create index, add documents, flush
        {
            let mut engine = SearchEngine::open(dir.path()).unwrap();
            engine.create_index("products").unwrap();
            engine.create_index("users").unwrap();

            let mut fields = HashMap::new();
            fields.insert("title".to_string(), serde_json::json!("bike"));
            let doc = Document::new("doc1", fields).unwrap();
            engine.add_document("products", doc).unwrap();

            let mut fields = HashMap::new();
            fields.insert("name".to_string(), serde_json::json!("Alice"));
            let doc = Document::new("user1", fields).unwrap();
            engine.add_document("users", doc).unwrap();

            engine.flush().unwrap();
        }

        // Session 2: reopen and verify recovery
        {
            let engine = SearchEngine::open(dir.path()).unwrap();
            let indexes = engine.list_indexes();
            assert!(indexes.contains(&"products".to_string()));
            assert!(indexes.contains(&"users".to_string()));

            let results = engine.search("products", "bike").unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].document_id, "doc1");

            let results = engine.search("users", "Alice").unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].document_id, "user1");
        }
    }

    #[test]
    fn persistent_engine_wal_replay() {
        let dir = tempfile::tempdir().unwrap();

        // Session 1: create + add without flush
        {
            let mut engine = SearchEngine::open(dir.path()).unwrap();
            engine.create_index("test").unwrap();

            let mut fields = HashMap::new();
            fields.insert("title".to_string(), serde_json::json!("hello"));
            let doc = Document::new("doc1", fields).unwrap();
            engine.add_document("test", doc).unwrap();

            // No flush — data is only in WAL
        }

        // Session 2: reopen — WAL should replay
        {
            let engine = SearchEngine::open(dir.path()).unwrap();
            assert!(engine.list_indexes().contains(&"test".to_string()));
            let results = engine.search("test", "hello").unwrap();
            assert_eq!(results.len(), 1);
        }
    }

    #[test]
    fn persistent_engine_multiple_sessions() {
        let dir = tempfile::tempdir().unwrap();

        let docs = ["doc1", "doc2", "doc3"];

        for (i, doc_id) in docs.iter().enumerate() {
            let mut engine = SearchEngine::open(dir.path()).unwrap();

            // Create index on first session only
            if i == 0 {
                engine.create_index("test").unwrap();
            }

            let mut fields = HashMap::new();
            fields.insert("title".to_string(), serde_json::json!(doc_id));
            let doc = Document::new(*doc_id, fields).unwrap();
            engine.add_document("test", doc).unwrap();
            engine.flush().unwrap();
        }

        // Final verification — all docs should be recoverable
        let engine = SearchEngine::open(dir.path()).unwrap();
        for doc_id in &docs {
            assert!(
                engine.get_document("test", doc_id).is_ok(),
                "document '{doc_id}' should be recoverable"
            );
        }
    }
}
