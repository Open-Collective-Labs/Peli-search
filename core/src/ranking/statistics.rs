use std::collections::HashMap;

use crate::tokenizer::tokenize;

/// Statistics about a single term across the collection.
#[derive(Debug, Clone, PartialEq)]
pub struct TermStats {
    /// Total number of times the term appears across all documents.
    pub term_frequency: u64,
    /// Number of documents that contain the term at least once.
    pub document_frequency: u64,
}

/// Collection-level statistics required for BM25 scoring.
#[derive(Debug, Clone)]
pub struct CollectionStats {
    /// Total number of documents in the collection.
    total_documents: u64,
    /// Sum of all document lengths (in tokens).
    total_document_length: u64,
    /// Per-term statistics: term -> TermStats.
    term_stats: HashMap<String, TermStats>,
    /// Per-doc per-term frequencies: term -> (doc_id -> frequency).
    term_doc_frequencies: HashMap<String, HashMap<String, u64>>,
    /// Per-document token counts: doc_id -> length.
    doc_lengths: HashMap<String, u64>,
}

impl CollectionStats {
    /// Create a new empty `CollectionStats`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::statistics::CollectionStats;
    ///
    /// let stats = CollectionStats::new();
    /// assert_eq!(stats.total_documents(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            total_documents: 0,
            total_document_length: 0,
            term_stats: HashMap::new(),
            term_doc_frequencies: HashMap::new(),
            doc_lengths: HashMap::new(),
        }
    }

    /// Update statistics when a document is indexed.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::statistics::CollectionStats;
    ///
    /// let mut stats = CollectionStats::new();
    /// stats.update_document("doc1", "hello world hello");
    /// assert_eq!(stats.total_documents(), 1);
    /// assert!((stats.average_document_length() - 3.0).abs() < 1e-6);
    /// ```
    pub fn update_document(&mut self, id: &str, text: &str) {
        let tokens = tokenize(text);
        let doc_length = tokens.len() as u64;

        // Count term frequencies within this document
        let mut local_freqs: HashMap<String, u64> = HashMap::new();
        for token in &tokens {
            *local_freqs.entry(token.clone()).or_insert(0) += 1;
        }

        // Update collection-level term stats
        for (term, freq_in_doc) in &local_freqs {
            let stats = self.term_stats.entry(term.clone()).or_insert(TermStats {
                term_frequency: 0,
                document_frequency: 0,
            });
            stats.term_frequency += freq_in_doc;
            stats.document_frequency += 1;

            self.term_doc_frequencies
                .entry(term.clone())
                .or_default()
                .insert(id.to_string(), *freq_in_doc);
        }

        self.total_documents += 1;
        self.total_document_length += doc_length;
        self.doc_lengths.insert(id.to_string(), doc_length);
    }

    /// Update statistics when a document is removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::statistics::CollectionStats;
    ///
    /// let mut stats = CollectionStats::new();
    /// stats.update_document("doc1", "hello world");
    /// stats.remove_document("doc1");
    /// assert_eq!(stats.total_documents(), 0);
    /// ```
    pub fn remove_document(&mut self, id: &str) {
        let doc_length = match self.doc_lengths.remove(id) {
            Some(len) => len,
            None => return,
        };

        // Collect terms associated with this document
        let mut affected_terms: Vec<String> = Vec::new();
        for (term, doc_map) in &self.term_doc_frequencies {
            if doc_map.contains_key(id) {
                affected_terms.push(term.clone());
            }
        }

        for term in &affected_terms {
            if let Some(doc_map) = self.term_doc_frequencies.get_mut(term) {
                if let Some(freq_in_doc) = doc_map.remove(id) {
                    if let Some(stats) = self.term_stats.get_mut(term) {
                        stats.term_frequency = stats.term_frequency.saturating_sub(freq_in_doc);
                        stats.document_frequency = stats.document_frequency.saturating_sub(1);
                        if stats.document_frequency == 0 {
                            self.term_stats.remove(term);
                        }
                    }
                }
                if doc_map.is_empty() {
                    self.term_doc_frequencies.remove(term);
                }
            }
        }

        self.total_documents = self.total_documents.saturating_sub(1);
        self.total_document_length = self.total_document_length.saturating_sub(doc_length);
    }

    /// Get statistics for a specific term.
    ///
    /// Returns `None` if the term has not been indexed.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::statistics::CollectionStats;
    ///
    /// let mut stats = CollectionStats::new();
    /// stats.update_document("doc1", "hello hello world");
    ///
    /// let hello_stats = stats.get_term_stats("hello").unwrap();
    /// assert_eq!(hello_stats.term_frequency, 2);
    /// assert_eq!(hello_stats.document_frequency, 1);
    ///
    /// assert!(stats.get_term_stats("missing").is_none());
    /// ```
    pub fn get_term_stats(&self, term: &str) -> Option<&TermStats> {
        self.term_stats.get(term)
    }

    /// Get the average document length across the collection.
    ///
    /// Returns `0.0` if there are no documents.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::ranking::statistics::CollectionStats;
    ///
    /// let mut stats = CollectionStats::new();
    /// stats.update_document("doc1", "hello world");
    /// stats.update_document("doc2", "hello there world");
    /// let avg = stats.average_document_length();
    /// assert!((avg - 2.5).abs() < 1e-6);
    /// ```
    pub fn average_document_length(&self) -> f64 {
        if self.total_documents == 0 {
            0.0
        } else {
            self.total_document_length as f64 / self.total_documents as f64
        }
    }

    /// Total number of documents in the collection.
    pub fn total_documents(&self) -> u64 {
        self.total_documents
    }

    /// Get the token length of a specific document.
    pub fn get_doc_length(&self, id: &str) -> Option<u64> {
        self.doc_lengths.get(id).copied()
    }

    /// Get the frequency of a term within a specific document.
    pub fn get_term_frequency_in_doc(&self, term: &str, doc_id: &str) -> u64 {
        self.term_doc_frequencies
            .get(term)
            .and_then(|doc_map| doc_map.get(doc_id))
            .copied()
            .unwrap_or(0)
    }
}

impl Default for CollectionStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_update_after_indexing() {
        let mut stats = CollectionStats::new();
        stats.update_document("doc1", "hello world hello");

        assert_eq!(stats.total_documents(), 1);
        assert!((stats.average_document_length() - 3.0).abs() < 1e-6);

        let hello_stats = stats.get_term_stats("hello").unwrap();
        assert_eq!(hello_stats.term_frequency, 2);
        assert_eq!(hello_stats.document_frequency, 1);

        let world_stats = stats.get_term_stats("world").unwrap();
        assert_eq!(world_stats.term_frequency, 1);
        assert_eq!(world_stats.document_frequency, 1);
    }

    #[test]
    fn statistics_update_after_deletion() {
        let mut stats = CollectionStats::new();
        stats.update_document("doc1", "hello world");
        stats.update_document("doc2", "hello there");
        stats.remove_document("doc1");

        assert_eq!(stats.total_documents(), 1);
        assert!((stats.average_document_length() - 2.0).abs() < 1e-6);

        let hello_stats = stats.get_term_stats("hello").unwrap();
        assert_eq!(hello_stats.term_frequency, 1);
        assert_eq!(hello_stats.document_frequency, 1);

        assert!(stats.get_term_stats("world").is_none());
    }

    #[test]
    fn empty_collection_handled() {
        let stats = CollectionStats::new();

        assert_eq!(stats.total_documents(), 0);
        assert!((stats.average_document_length() - 0.0).abs() < 1e-6);
        assert!(stats.get_term_stats("anything").is_none());
    }

    #[test]
    fn multiple_documents_same_term() {
        let mut stats = CollectionStats::new();
        stats.update_document("doc1", "rust programming");
        stats.update_document("doc2", "rust lang");
        stats.update_document("doc3", "rust");

        let rust_stats = stats.get_term_stats("rust").unwrap();
        assert_eq!(rust_stats.term_frequency, 3);
        assert_eq!(rust_stats.document_frequency, 3);
    }

    #[test]
    fn remove_nonexistent_document_is_noop() {
        let mut stats = CollectionStats::new();
        stats.update_document("doc1", "hello");
        stats.remove_document("nonexistent");
        assert_eq!(stats.total_documents(), 1);
    }

    #[test]
    fn average_document_length_multiple_docs() {
        let mut stats = CollectionStats::new();
        stats.update_document("doc1", "a b c");
        stats.update_document("doc2", "d e");
        stats.update_document("doc3", "f");
        assert!((stats.average_document_length() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn remove_and_re_add_document() {
        let mut stats = CollectionStats::new();
        stats.update_document("doc1", "hello world");
        stats.remove_document("doc1");
        stats.update_document("doc1", "hello again");

        assert_eq!(stats.total_documents(), 1);
        let hello_stats = stats.get_term_stats("hello").unwrap();
        assert_eq!(hello_stats.term_frequency, 1);
        assert!(stats.get_term_stats("world").is_none());
        let again_stats = stats.get_term_stats("again").unwrap();
        assert_eq!(again_stats.term_frequency, 1);
    }

    #[test]
    fn term_frequency_in_doc() {
        let mut stats = CollectionStats::new();
        stats.update_document("doc1", "hello hello world");
        stats.update_document("doc2", "hello there");

        assert_eq!(stats.get_term_frequency_in_doc("hello", "doc1"), 2);
        assert_eq!(stats.get_term_frequency_in_doc("hello", "doc2"), 1);
        assert_eq!(stats.get_term_frequency_in_doc("world", "doc1"), 1);
        assert_eq!(stats.get_term_frequency_in_doc("world", "doc2"), 0);
    }

    #[test]
    fn get_doc_length() {
        let mut stats = CollectionStats::new();
        stats.update_document("doc1", "a b c d");
        assert_eq!(stats.get_doc_length("doc1"), Some(4));
        assert_eq!(stats.get_doc_length("nonexistent"), None);
    }

    #[test]
    fn duplicate_terms_same_document() {
        let mut stats = CollectionStats::new();
        stats.update_document("doc1", "hello hello");
        stats.update_document("doc2", "hello");

        let hello_stats = stats.get_term_stats("hello").unwrap();
        assert_eq!(hello_stats.term_frequency, 3);
        assert_eq!(hello_stats.document_frequency, 2);
    }
}
