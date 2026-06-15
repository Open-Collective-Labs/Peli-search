use std::collections::HashMap;

use crate::document::Document;
use crate::error::SearchError;
use crate::index::Index;
use crate::schema::Mapping;
use crate::types::SearchResult;

/// Manages multiple named indexes.
///
/// Each index is identified by a unique name and has its own documents,
/// inverted index, statistics, and schema mapping.
///
/// # Examples
///
/// ```
/// use pelisearch_core::index::IndexManager;
/// use pelisearch_core::schema::{Mapping, Field, FieldType};
///
/// let mut manager = IndexManager::new();
///
/// let product_mapping = Mapping::new(vec![
///     Field::new("title", FieldType::Text, true),
/// ]);
/// manager.create_index("products", product_mapping).unwrap();
///
/// assert!(manager.index_exists("products"));
/// assert!(!manager.index_exists("nonexistent"));
/// ```
pub struct IndexManager {
    indexes: HashMap<String, Index>,
}

impl IndexManager {
    /// Create a new empty `IndexManager`.
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
        }
    }

    /// Create a new named index with the given mapping.
    ///
    /// Returns an error if an index with the same name already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::IndexManager;
    /// use pelisearch_core::schema::Mapping;
    ///
    /// let mut manager = IndexManager::new();
    /// manager.create_index("products", Mapping::new(vec![])).unwrap();
    /// ```
    pub fn create_index(
        &mut self,
        name: impl Into<String>,
        mapping: Mapping,
    ) -> Result<(), SearchError> {
        let name: String = name.into();
        if self.indexes.contains_key(&name) {
            return Err(SearchError::Internal(format!(
                "index '{name}' already exists"
            )));
        }
        self.indexes.insert(name.clone(), Index::new(name, mapping));
        Ok(())
    }

    /// Get a reference to an index by name.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::IndexManager;
    /// use pelisearch_core::schema::Mapping;
    ///
    /// let mut manager = IndexManager::new();
    /// manager.create_index("products", Mapping::new(vec![])).unwrap();
    /// let index = manager.get_index("products").unwrap();
    /// assert_eq!(index.name(), "products");
    /// ```
    pub fn get_index(&self, name: &str) -> Result<&Index, SearchError> {
        self.indexes
            .get(name)
            .ok_or_else(|| SearchError::Internal(format!("index '{name}' not found")))
    }

    /// Get a mutable reference to an index by name.
    pub fn get_index_mut(&mut self, name: &str) -> Result<&mut Index, SearchError> {
        self.indexes
            .get_mut(name)
            .ok_or_else(|| SearchError::Internal(format!("index '{name}' not found")))
    }

    /// Check whether an index exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::IndexManager;
    /// use pelisearch_core::schema::Mapping;
    ///
    /// let mut manager = IndexManager::new();
    /// manager.create_index("products", Mapping::new(vec![])).unwrap();
    /// assert!(manager.index_exists("products"));
    /// assert!(!manager.index_exists("users"));
    /// ```
    pub fn index_exists(&self, name: &str) -> bool {
        self.indexes.contains_key(name)
    }

    /// Remove an index by name.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::IndexManager;
    /// use pelisearch_core::schema::Mapping;
    ///
    /// let mut manager = IndexManager::new();
    /// manager.create_index("products", Mapping::new(vec![])).unwrap();
    /// manager.drop_index("products").unwrap();
    /// assert!(!manager.index_exists("products"));
    /// ```
    pub fn drop_index(&mut self, name: &str) -> Result<(), SearchError> {
        if !self.indexes.contains_key(name) {
            return Err(SearchError::Internal(format!(
                "index '{name}' not found"
            )));
        }
        self.indexes.remove(name);
        Ok(())
    }

    /// Add a document to a specific index.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::index::IndexManager;
    /// use pelisearch_core::schema::Mapping;
    ///
    /// let mut manager = IndexManager::new();
    /// manager.create_index("products", Mapping::new(vec![])).unwrap();
    ///
    /// let doc = Document::new("doc1", HashMap::new()).unwrap();
    /// manager.add_document("products", doc).unwrap();
    /// ```
    pub fn add_document(
        &mut self,
        index_name: &str,
        document: Document,
    ) -> Result<(), SearchError> {
        let index = self.get_index_mut(index_name)?;
        index.add_document(document)
    }

    /// Remove a document from a specific index.
    pub fn remove_document(
        &mut self,
        index_name: &str,
        doc_id: &str,
    ) -> Result<(), SearchError> {
        let index = self.get_index_mut(index_name)?;
        index.remove_document(doc_id)
    }

    /// Search a specific index.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::index::IndexManager;
    /// use pelisearch_core::schema::{Mapping, Field, FieldType};
    ///
    /// let mut manager = IndexManager::new();
    /// let mapping = Mapping::new(vec![
    ///     Field::new("title", FieldType::Text, true),
    /// ]);
    /// manager.create_index("products", mapping).unwrap();
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("electric bike"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// manager.add_document("products", doc).unwrap();
    ///
    /// let results = manager.search("products", "bike").unwrap();
    /// assert_eq!(results.len(), 1);
    /// ```
    pub fn search(
        &self,
        index_name: &str,
        query: &str,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let index = self.get_index(index_name)?;
        Ok(index.search(query))
    }

    /// List all index names managed by this manager.
    pub fn index_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.indexes.keys().cloned().collect();
        names.sort();
        names
    }
}

impl Default for IndexManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Field, FieldType};

    #[test]
    fn create_and_list_indexes() {
        let mut manager = IndexManager::new();
        manager
            .create_index("products", Mapping::new(vec![]))
            .unwrap();
        manager
            .create_index("users", Mapping::new(vec![]))
            .unwrap();

        let names = manager.index_names();
        assert_eq!(names, vec!["products", "users"]);
    }

    #[test]
    fn create_duplicate_index_fails() {
        let mut manager = IndexManager::new();
        manager
            .create_index("products", Mapping::new(vec![]))
            .unwrap();
        let err = manager
            .create_index("products", Mapping::new(vec![]))
            .unwrap_err();
        assert!(matches!(err, SearchError::Internal(_)));
    }

    #[test]
    fn get_nonexistent_index_fails() {
        let manager = IndexManager::new();
        let err = manager.get_index("nonexistent").unwrap_err();
        assert!(matches!(err, SearchError::Internal(_)));
    }

    #[test]
    fn drop_index() {
        let mut manager = IndexManager::new();
        manager
            .create_index("products", Mapping::new(vec![]))
            .unwrap();
        manager.drop_index("products").unwrap();
        assert!(!manager.index_exists("products"));
    }

    #[test]
    fn drop_nonexistent_index_fails() {
        let mut manager = IndexManager::new();
        let err = manager.drop_index("nonexistent").unwrap_err();
        assert!(matches!(err, SearchError::Internal(_)));
    }

    #[test]
    fn add_and_search_documents_through_manager() {
        let mut manager = IndexManager::new();
        let mapping = Mapping::new(vec![Field::new("title", FieldType::Text, true)]);
        manager.create_index("products", mapping).unwrap();

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("electric bike"));
        let doc = Document::new("doc1", fields).unwrap();
        manager.add_document("products", doc).unwrap();

        let results = manager.search("products", "bike").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc1");
    }

    #[test]
    fn add_to_nonexistent_index_fails() {
        let mut manager = IndexManager::new();
        let doc = Document::new("doc1", HashMap::new()).unwrap();
        let err = manager.add_document("nonexistent", doc).unwrap_err();
        assert!(matches!(err, SearchError::Internal(_)));
    }
}
