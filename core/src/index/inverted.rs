use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::tokenizer::tokenize;

/// A simple in-memory inverted index mapping terms to document IDs.
///
/// Now includes a positions map for phrase/proximity queries.
///
/// # Examples
///
/// ```
/// use pelisearch_core::index::InvertedIndex;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvertedIndex {
    postings: HashMap<String, Vec<String>>,
    doc_terms: HashMap<String, Vec<String>>,
    /// term -> (doc_id -> positions)
    positions: HashMap<String, HashMap<String, Vec<usize>>>,
}

impl InvertedIndex {
    /// Create a new empty `InvertedIndex`.
    pub fn new() -> Self {
        Self {
            postings: HashMap::new(),
            doc_terms: HashMap::new(),
            positions: HashMap::new(),
        }
    }

    /// Add a document's text to the index, tracking token positions.
    ///
    /// The text is tokenized and each token is mapped to the document ID
    /// along with its position (for phrase/proximity queries).
    /// Returns an error if the document ID is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::InvertedIndex;
    ///
    /// let mut index = InvertedIndex::new();
    /// index.add_document("doc1", "the quick brown fox").unwrap();
    /// assert!(index.get_postings("quick").unwrap().contains(&"doc1".to_string()));
    /// ```
    pub fn add_document(&mut self, id: &str, text: &str) -> Result<(), crate::error::SearchError> {
        if id.is_empty() {
            return Err(crate::error::SearchError::InvalidDocumentId(
                "document ID must not be empty".to_string(),
            ));
        }

        let tokens = tokenize(text);
        let mut unique_terms = Vec::new();

        for (pos, token) in tokens.iter().enumerate() {
            let entry = self.postings.entry(token.clone()).or_default();
            if !entry.contains(&id.to_string()) {
                entry.push(id.to_string());
            }
            if !unique_terms.contains(token) {
                unique_terms.push(token.clone());
            }

            self.positions
                .entry(token.clone())
                .or_default()
                .entry(id.to_string())
                .or_default()
                .push(pos);
        }

        self.doc_terms.insert(id.to_string(), unique_terms);
        Ok(())
    }

    /// Remove a document from the index, including its positions.
    ///
    /// All postings and positions associated with the document are removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::InvertedIndex;
    ///
    /// let mut index = InvertedIndex::new();
    /// index.add_document("doc1", "hello world").unwrap();
    /// index.remove_document("doc1");
    /// assert_eq!(index.get_postings("hello"), None);
    /// ```
    pub fn remove_document(&mut self, id: &str) {
        if let Some(terms) = self.doc_terms.remove(id) {
            for term in terms {
                if let Some(postings) = self.postings.get_mut(&term) {
                    postings.retain(|doc_id| doc_id != id);
                    if postings.is_empty() {
                        self.postings.remove(&term);
                    }
                }
                if let Some(doc_positions) = self.positions.get_mut(&term) {
                    doc_positions.remove(id);
                    if doc_positions.is_empty() {
                        self.positions.remove(&term);
                    }
                }
            }
        }
    }

    /// Get the list of document IDs containing the given term.
    ///
    /// Returns `None` if the term has no postings.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::InvertedIndex;
    ///
    /// let mut index = InvertedIndex::new();
    /// index.add_document("doc1", "hello world").unwrap();
    /// index.add_document("doc2", "hello there").unwrap();
    /// let postings = index.get_postings("hello").unwrap();
    /// assert_eq!(postings.len(), 2);
    /// assert_eq!(index.get_postings("missing"), None);
    /// ```
    pub fn get_postings(&self, term: &str) -> Option<&Vec<String>> {
        self.postings.get(term)
    }

    /// Get the positions of a term within a specific document.
    ///
    /// Returns `None` if the term has no positions in the document.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::index::InvertedIndex;
    ///
    /// let mut index = InvertedIndex::new();
    /// index.add_document("doc1", "hello world hello").unwrap();
    /// let positions = index.get_positions("hello", "doc1").unwrap();
    /// assert_eq!(positions, &vec![0, 2]);
    /// ```
    pub fn get_positions(&self, term: &str, doc_id: &str) -> Option<&Vec<usize>> {
        self.positions.get(term)?.get(doc_id)
    }

    /// Get all terms in the index.
    pub fn terms(&self) -> impl Iterator<Item = &String> {
        self.postings.keys()
    }

    /// Get all document IDs containing the given term.
    pub fn get_doc_ids(&self, term: &str) -> Option<impl Iterator<Item = &String>> {
        self.postings.get(term).map(|v| v.iter())
    }
}

impl Default for InvertedIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documents_become_searchable() {
        let mut index = InvertedIndex::new();
        index.add_document("doc1", "hello world").unwrap();
        index.add_document("doc2", "goodbye world").unwrap();

        let hello_postings = index.get_postings("hello").unwrap();
        assert_eq!(hello_postings, &vec!["doc1".to_string()]);

        let world_postings = index.get_postings("world").unwrap();
        assert_eq!(world_postings.len(), 2);
        assert!(world_postings.contains(&"doc1".to_string()));
        assert!(world_postings.contains(&"doc2".to_string()));
    }

    #[test]
    fn removing_document_removes_postings() {
        let mut index = InvertedIndex::new();
        index.add_document("doc1", "hello world").unwrap();
        index.add_document("doc2", "hello there").unwrap();
        index.remove_document("doc1");

        let hello_postings = index.get_postings("hello").unwrap();
        assert_eq!(hello_postings, &vec!["doc2".to_string()]);

        assert_eq!(index.get_postings("world"), None);
    }

    #[test]
    fn duplicate_terms_handled_correctly() {
        let mut index = InvertedIndex::new();
        index.add_document("doc1", "hello hello hello").unwrap();

        let hello_postings = index.get_postings("hello").unwrap();
        assert_eq!(hello_postings.len(), 1);
        assert_eq!(hello_postings, &vec!["doc1".to_string()]);
    }

    #[test]
    fn missing_terms_return_empty() {
        let index = InvertedIndex::new();
        assert_eq!(index.get_postings("nonexistent"), None);
    }

    #[test]
    fn add_multiple_documents_same_term() {
        let mut index = InvertedIndex::new();
        index.add_document("doc1", "rust").unwrap();
        index.add_document("doc2", "rust").unwrap();
        index.add_document("doc3", "rust").unwrap();

        let postings = index.get_postings("rust").unwrap();
        assert_eq!(postings.len(), 3);
    }

    #[test]
    fn remove_nonexistent_document_is_noop() {
        let mut index = InvertedIndex::new();
        index.add_document("doc1", "hello").unwrap();
        index.remove_document("nonexistent");
        assert!(index.get_postings("hello").is_some());
    }

    #[test]
    fn empty_id_returns_error() {
        let mut index = InvertedIndex::new();
        let err = index.add_document("", "hello world").unwrap_err();
        assert!(matches!(err, crate::error::SearchError::InvalidDocumentId(_)));
    }

    #[test]
    fn index_after_remove_and_re_add() {
        let mut index = InvertedIndex::new();
        index.add_document("doc1", "hello world").unwrap();
        index.remove_document("doc1");
        index.add_document("doc1", "hello again").unwrap();

        let hello_postings = index.get_postings("hello").unwrap();
        assert_eq!(hello_postings, &vec!["doc1".to_string()]);
        assert_eq!(index.get_postings("world"), None);
        let again_postings = index.get_postings("again").unwrap();
        assert_eq!(again_postings, &vec!["doc1".to_string()]);
    }
}
