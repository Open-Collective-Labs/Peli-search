use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::index::InvertedIndex;

use super::compressed_posting_list::CompressedPostingList;
use super::doc_id_mapper::DocIdMapper;

/// An inverted index backed by compressed posting lists.
///
/// Unlike the standard `InvertedIndex` which stores posting lists as
/// `Vec<String>` (high memory), this index stores them as delta+varint
/// compressed `Vec<u8>` internally. The compression is transparent —
/// callers interact with string doc IDs as usual.
///
/// # Memory Savings
///
/// For a term that appears in 1000 consecutive documents:
/// - Standard: 1000 × (string length + Vec overhead) bytes
/// - Compressed: ~1000 bytes (deltas of 1)
///
/// # Usage
///
/// ```
/// use pelisearch_core::compression::CompressedIndex;
///
/// let mut idx = CompressedIndex::new();
/// idx.add_document("doc1", "hello world").unwrap();
/// assert!(idx.contains_term("hello"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedIndex {
    /// Term → compressed posting list of numeric doc IDs.
    postings: HashMap<String, CompressedPostingList>,
    /// Document → list of terms (for removal).
    doc_terms: HashMap<String, Vec<String>>,
    /// String ↔ numeric ID mapping.
    #[serde(skip)]
    mapper: DocIdMapper,
}

impl CompressedIndex {
    /// Create a new empty `CompressedIndex`.
    pub fn new() -> Self {
        Self {
            postings: HashMap::new(),
            doc_terms: HashMap::new(),
            mapper: DocIdMapper::new(),
        }
    }

    /// Add a document's text to the compressed index.
    ///
    /// The text is tokenized and each token is mapped to the document's
    /// numeric ID. The posting list is stored in delta+varint compressed
    /// format.
    pub fn add_document(&mut self, id: &str, text: &str) -> Result<(), crate::error::SearchError> {
        if id.is_empty() {
            return Err(crate::error::SearchError::InvalidDocumentId(
                "document ID must not be empty".to_string(),
            ));
        }

        let numeric_id = self.mapper.get_or_assign(id);
        let tokens = crate::tokenizer::tokenize(text);
        let mut unique_terms = Vec::new();

        for token in &tokens {
            let entry = self.postings.entry(token.clone()).or_default();
            let ids = entry.decode();
            if !ids.contains(&numeric_id) {
                let mut new_ids = ids;
                new_ids.push(numeric_id);
                new_ids.sort();
                *entry = CompressedPostingList::from_sorted(&new_ids);
            }
            if !unique_terms.contains(token) {
                unique_terms.push(token.clone());
            }
        }

        self.doc_terms.insert(id.to_string(), unique_terms);
        Ok(())
    }

    /// Remove a document from the index.
    pub fn remove_document(&mut self, id: &str) {
        if let Some(numeric_id) = self.mapper.get_numeric(id) {
            if let Some(terms) = self.doc_terms.remove(id) {
                for term in terms {
                    if let Some(entry) = self.postings.get_mut(&term) {
                        let ids: Vec<u64> = entry
                            .decode()
                            .into_iter()
                            .filter(|&nid| nid != numeric_id)
                            .collect();
                        if ids.is_empty() {
                            self.postings.remove(&term);
                        } else {
                            *entry = CompressedPostingList::from_sorted(&ids);
                        }
                    }
                }
            }
        }
    }

    /// Get the uncompressed posting list for a term.
    ///
    /// Returns a list of string document IDs (decompressed from the
    /// internal numeric storage).
    pub fn get_postings(&self, term: &str) -> Option<Vec<String>> {
        self.postings.get(term).map(|compressed| {
            compressed
                .decode()
                .iter()
                .filter_map(|&nid| self.mapper.get_string(nid).map(|s| s.to_string()))
                .collect()
        })
    }

    /// Check whether a term exists in the index.
    pub fn contains_term(&self, term: &str) -> bool {
        self.postings.contains_key(term)
    }

    /// Return the number of unique terms in the index.
    pub fn num_terms(&self) -> usize {
        self.postings.len()
    }

    /// Return the total number of documents indexed.
    pub fn num_documents(&self) -> usize {
        self.mapper.len()
    }

    /// Return the compressed size of all posting lists (bytes).
    pub fn compressed_size(&self) -> usize {
        self.postings
            .values()
            .map(|p| p.compressed_size())
            .sum()
    }

    /// Estimate the uncompressed size of all posting lists (bytes).
    pub fn uncompressed_size(&self) -> usize {
        self.postings
            .values()
            .map(|p| p.uncompressed_size())
            .sum()
    }

    /// Convert this compressed index into a standard `InvertedIndex`.
    ///
    /// This is useful for compatibility with search code that expects
    /// a regular `InvertedIndex`.
    pub fn to_inverted_index(&self) -> InvertedIndex {
        let mut idx = InvertedIndex::new();
        for term in self.postings.keys() {
            if let Some(postings) = self.get_postings(term) {
                for doc_id in postings {
                    let _ = idx.add_document(&doc_id, term);
                }
            }
        }
        idx
    }
}

impl Default for CompressedIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_retrieve() {
        let mut idx = CompressedIndex::new();
        idx.add_document("doc1", "hello world").unwrap();
        idx.add_document("doc2", "hello there").unwrap();

        let postings = idx.get_postings("hello").unwrap();
        assert_eq!(postings.len(), 2);
        assert!(postings.contains(&"doc1".to_string()));
        assert!(postings.contains(&"doc2".to_string()));

        let world_postings = idx.get_postings("world").unwrap();
        assert_eq!(world_postings, vec!["doc1"]);
    }

    #[test]
    fn remove_document() {
        let mut idx = CompressedIndex::new();
        idx.add_document("doc1", "hello world").unwrap();
        idx.add_document("doc2", "hello there").unwrap();
        idx.remove_document("doc1");

        let postings = idx.get_postings("hello").unwrap();
        assert_eq!(postings, vec!["doc2"]);
        assert!(idx.get_postings("world").is_none());
    }

    #[test]
    fn search_accuracy_preserved() {
        let mut idx = CompressedIndex::new();
        idx.add_document("doc1", "the quick brown fox").unwrap();
        idx.add_document("doc2", "jumps over the lazy dog").unwrap();

        let fox_postings = idx.get_postings("fox").unwrap();
        assert_eq!(fox_postings, vec!["doc1"]);

        let the_postings = idx.get_postings("the").unwrap();
        assert_eq!(the_postings.len(), 2);
    }

    #[test]
    fn compressed_size_smaller_than_uncompressed() {
        let mut idx = CompressedIndex::new();
        // Add many docs with the same term to get good compression
        for i in 0..1000 {
            let doc_id = format!("doc{i}");
            idx.add_document(&doc_id, "hello world").unwrap();
        }

        let compressed = idx.compressed_size();
        let uncompressed = idx.uncompressed_size();

        assert!(
            compressed < uncompressed / 2,
            "compressed size should be significantly smaller: compressed={compressed}, uncompressed={uncompressed}"
        );
    }

    #[test]
    fn to_inverted_index_preserves_data() {
        let mut compressed = CompressedIndex::new();
        compressed.add_document("doc1", "hello world").unwrap();
        compressed.add_document("doc2", "hello there").unwrap();

        let standard = compressed.to_inverted_index();
        assert!(standard.get_postings("hello").is_some());
        assert_eq!(standard.get_postings("hello").unwrap().len(), 2);
        assert!(standard.get_postings("world").is_some());
    }

    #[test]
    fn empty_index() {
        let idx = CompressedIndex::new();
        assert_eq!(idx.num_documents(), 0);
        assert_eq!(idx.num_terms(), 0);
        assert!(idx.get_postings("anything").is_none());
    }

    #[test]
    fn contains_term() {
        let mut idx = CompressedIndex::new();
        idx.add_document("doc1", "hello world").unwrap();
        assert!(idx.contains_term("hello"));
        assert!(!idx.contains_term("nonexistent"));
    }

    #[test]
    fn num_documents_and_terms() {
        let mut idx = CompressedIndex::new();
        idx.add_document("doc1", "hello world").unwrap();
        idx.add_document("doc2", "hello there").unwrap();

        assert_eq!(idx.num_documents(), 2);
        assert_eq!(idx.num_terms(), 3); // hello, world, there
    }

    #[test]
    fn remove_nonexistent_noop() {
        let mut idx = CompressedIndex::new();
        idx.add_document("doc1", "hello").unwrap();
        idx.remove_document("nonexistent");
        assert_eq!(idx.num_documents(), 1);
    }

    #[test]
    fn large_term_posting_list() {
        let mut idx = CompressedIndex::new();
        let n = 5000;
        for i in 0..n {
            let doc_id = format!("doc{i}");
            idx.add_document(&doc_id, "common").unwrap();
        }

        let postings = idx.get_postings("common").unwrap();
        assert_eq!(postings.len(), n);
    }
}
