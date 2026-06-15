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
///
/// let mut manager = IndexManager::new();
/// manager.create_index("products").unwrap();
/// manager.create_index("articles").unwrap();
///
/// assert_eq!(manager.list_indexes(), vec!["articles", "products"]);
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

    /// Create a new named index with an empty schema mapping.
    ///
    /// Returns an error if an index with the same name already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::IndexManager;
    ///
    /// let mut manager = IndexManager::new();
    /// manager.create_index("products").unwrap();
    /// assert!(manager.get_index("products").is_ok());
    /// ```
    pub fn create_index(&mut self, name: impl Into<String>) -> Result<(), SearchError> {
        let name: String = name.into();
        if self.indexes.contains_key(&name) {
            return Err(SearchError::Internal(format!(
                "index '{name}' already exists"
            )));
        }
        self.indexes
            .insert(name.clone(), Index::new(name, Mapping::new(vec![])));
        Ok(())
    }

    /// Create a new named index with the given schema mapping.
    ///
    /// Returns an error if an index with the same name already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::IndexManager;
    /// use pelisearch_core::schema::{Mapping, Field, FieldType};
    ///
    /// let mut manager = IndexManager::new();
    /// let mapping = Mapping::new(vec![
    ///     Field::new("title", FieldType::Text, true),
    /// ]);
    /// manager.create_index_with_mapping("products", mapping).unwrap();
    /// ```
    pub fn create_index_with_mapping(
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

    /// Delete (remove) an index by name.
    ///
    /// Returns an error if the index does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::IndexManager;
    ///
    /// let mut manager = IndexManager::new();
    /// manager.create_index("products").unwrap();
    /// manager.delete_index("products").unwrap();
    /// assert!(!manager.get_index("products").is_ok());
    /// ```
    pub fn delete_index(&mut self, name: &str) -> Result<(), SearchError> {
        if !self.indexes.contains_key(name) {
            return Err(SearchError::Internal(format!(
                "index '{name}' not found"
            )));
        }
        self.indexes.remove(name);
        Ok(())
    }

    /// Get a reference to an index by name.
    ///
    /// Returns an error if the index does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::IndexManager;
    ///
    /// let mut manager = IndexManager::new();
    /// manager.create_index("products").unwrap();
    /// let index = manager.get_index("products").unwrap();
    /// assert_eq!(index.name(), "products");
    /// ```
    pub fn get_index(&self, name: &str) -> Result<&Index, SearchError> {
        self.indexes
            .get(name)
            .ok_or_else(|| SearchError::Internal(format!("index '{name}' not found")))
    }

    /// Get a mutable reference to an index by name.
    ///
    /// Returns an error if the index does not exist.
    pub fn get_index_mut(&mut self, name: &str) -> Result<&mut Index, SearchError> {
        self.indexes
            .get_mut(name)
            .ok_or_else(|| SearchError::Internal(format!("index '{name}' not found")))
    }

    /// Check whether an index exists.
    pub fn index_exists(&self, name: &str) -> bool {
        self.indexes.contains_key(name)
    }

    /// List all index names managed by this manager, sorted alphabetically.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::IndexManager;
    ///
    /// let mut manager = IndexManager::new();
    /// manager.create_index("z_products").unwrap();
    /// manager.create_index("a_articles").unwrap();
    ///
    /// let names = manager.list_indexes();
    /// assert_eq!(names, vec!["a_articles", "z_products"]);
    /// ```
    pub fn list_indexes(&self) -> Vec<String> {
        let mut names: Vec<String> = self.indexes.keys().cloned().collect();
        names.sort();
        names
    }

    /// Add a document to a specific index.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::index::IndexManager;
    ///
    /// let mut manager = IndexManager::new();
    /// manager.create_index("products").unwrap();
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
    /// manager.create_index_with_mapping("products", mapping).unwrap();
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
    fn create_with_default_mapping() {
        let mut manager = IndexManager::new();
        manager.create_index("products").unwrap();
        assert!(manager.index_exists("products"));
        assert!(manager.get_index("products").unwrap().mapping().fields().is_empty());
    }

    #[test]
    fn create_with_custom_mapping() {
        let mut manager = IndexManager::new();
        let mapping = Mapping::new(vec![Field::new("title", FieldType::Text, true)]);
        manager
            .create_index_with_mapping("products", mapping)
            .unwrap();
        assert!(manager.get_index("products").unwrap().mapping().field_exists("title"));
    }

    #[test]
    fn create_and_list_indexes() {
        let mut manager = IndexManager::new();
        manager.create_index("products").unwrap();
        manager.create_index("users").unwrap();
        assert_eq!(manager.list_indexes(), vec!["products", "users"]);
    }

    #[test]
    fn create_duplicate_index_fails() {
        let mut manager = IndexManager::new();
        manager.create_index("products").unwrap();
        let err = manager.create_index("products").unwrap_err();
        assert!(matches!(err, SearchError::Internal(_)));
    }

    #[test]
    fn get_nonexistent_index_fails() {
        let manager = IndexManager::new();
        let err = manager.get_index("nonexistent").unwrap_err();
        assert!(matches!(err, SearchError::Internal(_)));
    }

    #[test]
    fn delete_index() {
        let mut manager = IndexManager::new();
        manager.create_index("products").unwrap();
        manager.delete_index("products").unwrap();
        assert!(!manager.index_exists("products"));
        assert!(manager.list_indexes().is_empty());
    }

    #[test]
    fn delete_nonexistent_index_fails() {
        let mut manager = IndexManager::new();
        let err = manager.delete_index("nonexistent").unwrap_err();
        assert!(matches!(err, SearchError::Internal(_)));
    }

    #[test]
    fn list_indexes_empty() {
        let manager = IndexManager::new();
        assert!(manager.list_indexes().is_empty());
    }

    #[test]
    fn list_indexes_sorted() {
        let mut manager = IndexManager::new();
        manager.create_index("zebra").unwrap();
        manager.create_index("apple").unwrap();
        manager.create_index("mango").unwrap();
        assert_eq!(manager.list_indexes(), vec!["apple", "mango", "zebra"]);
    }

    #[test]
    fn add_and_search_documents_through_manager() {
        let mut manager = IndexManager::new();
        let mapping = Mapping::new(vec![Field::new("title", FieldType::Text, true)]);
        manager.create_index_with_mapping("products", mapping).unwrap();

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

    #[test]
    fn remove_document_through_manager() {
        let mut manager = IndexManager::new();
        manager.create_index("test").unwrap();

        let doc = Document::new("doc1", HashMap::new()).unwrap();
        manager.add_document("test", doc).unwrap();
        manager.remove_document("test", "doc1").unwrap();
        assert!(manager.get_index("test").unwrap().get_document("doc1").is_err());
    }

    #[test]
    fn search_after_delete_recreate() {
        let mut manager = IndexManager::new();
        manager.create_index("test").unwrap();
        manager.delete_index("test").unwrap();
        manager.create_index("test").unwrap();
        assert!(manager.index_exists("test"));
    }
}
