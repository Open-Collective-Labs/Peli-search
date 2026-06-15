use std::collections::HashMap;

/// Maps string document IDs to internal u64 numeric IDs.
///
/// Search engines internally use numeric doc IDs (not strings) for
/// efficient posting list storage. This mapper provides a bidirectional
/// mapping between the external string IDs (like "doc1", "product_abc")
/// and internal sequential u64 IDs.
///
/// # Memory
///
/// The mapping table grows linearly with the number of documents.
/// For 1M documents, the forward map (String → u64) uses roughly
/// the same memory as the string IDs themselves.
#[derive(Debug, Clone)]
pub struct DocIdMapper {
    /// Forward mapping: string ID → numeric ID.
    forward: HashMap<String, u64>,
    /// Reverse mapping: numeric ID → string ID.
    reverse: Vec<String>,
}

impl DocIdMapper {
    /// Create an empty mapper.
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            reverse: Vec::new(),
        }
    }

    /// Assign or lookup a numeric ID for a string document ID.
    ///
    /// If the string ID is already mapped, returns the existing numeric ID.
    /// Otherwise, assigns the next available numeric ID.
    pub fn get_or_assign(&mut self, doc_id: &str) -> u64 {
        if let Some(&id) = self.forward.get(doc_id) {
            return id;
        }
        let id = self.reverse.len() as u64;
        self.forward.insert(doc_id.to_string(), id);
        self.reverse.push(doc_id.to_string());
        id
    }

    /// Lookup the string ID for a numeric ID.
    ///
    /// Returns `None` if the numeric ID has no mapping.
    pub fn get_string(&self, numeric_id: u64) -> Option<&str> {
        self.reverse
            .get(numeric_id as usize)
            .map(|s| s.as_str())
    }

    /// Lookup the numeric ID for a string ID.
    ///
    /// Returns `None` if the string ID has not been mapped.
    pub fn get_numeric(&self, doc_id: &str) -> Option<u64> {
        self.forward.get(doc_id).copied()
    }

    /// Check whether a string ID has been assigned a numeric ID.
    pub fn contains(&self, doc_id: &str) -> bool {
        self.forward.contains_key(doc_id)
    }

    /// Return the number of mapped documents.
    pub fn len(&self) -> usize {
        self.reverse.len()
    }

    /// Check if no documents are mapped.
    pub fn is_empty(&self) -> bool {
        self.reverse.is_empty()
    }

    /// Return all string IDs in order of their numeric assignment.
    pub fn all_string_ids(&self) -> Vec<&str> {
        self.reverse.iter().map(|s| s.as_str()).collect()
    }

    /// Return all numeric IDs that have been assigned.
    pub fn all_numeric_ids(&self) -> Vec<u64> {
        (0..self.reverse.len() as u64).collect()
    }
}

impl Default for DocIdMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assign_and_lookup() {
        let mut mapper = DocIdMapper::new();
        let id1 = mapper.get_or_assign("doc1");
        let id2 = mapper.get_or_assign("doc2");

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(mapper.get_string(0), Some("doc1"));
        assert_eq!(mapper.get_string(1), Some("doc2"));
    }

    #[test]
    fn existing_id_returns_same_numeric() {
        let mut mapper = DocIdMapper::new();
        let id1 = mapper.get_or_assign("doc1");
        let id2 = mapper.get_or_assign("doc1");
        assert_eq!(id1, id2);
        assert_eq!(mapper.len(), 1);
    }

    #[test]
    fn contains_check() {
        let mut mapper = DocIdMapper::new();
        assert!(!mapper.contains("doc1"));
        mapper.get_or_assign("doc1");
        assert!(mapper.contains("doc1"));
    }

    #[test]
    fn get_numeric() {
        let mut mapper = DocIdMapper::new();
        mapper.get_or_assign("product_123");
        assert_eq!(mapper.get_numeric("product_123"), Some(0));
        assert_eq!(mapper.get_numeric("nonexistent"), None);
    }

    #[test]
    fn get_string_nonexistent() {
        let mapper = DocIdMapper::new();
        assert_eq!(mapper.get_string(0), None);
        assert_eq!(mapper.get_string(999), None);
    }

    #[test]
    fn empty_mapper() {
        let mapper = DocIdMapper::new();
        assert!(mapper.is_empty());
        assert_eq!(mapper.len(), 0);
        assert!(mapper.all_string_ids().is_empty());
        assert!(mapper.all_numeric_ids().is_empty());
    }

    #[test]
    fn all_ids_consistent() {
        let mut mapper = DocIdMapper::new();
        mapper.get_or_assign("a");
        mapper.get_or_assign("b");
        mapper.get_or_assign("c");

        let strings = mapper.all_string_ids();
        assert_eq!(strings, vec!["a", "b", "c"]);

        let numerics = mapper.all_numeric_ids();
        assert_eq!(numerics, vec![0, 1, 2]);
    }

    #[test]
    fn reuse_after_mapping() {
        let mut mapper = DocIdMapper::new();
        mapper.get_or_assign("first");
        mapper.get_or_assign("second");

        assert_eq!(mapper.get_numeric("first"), Some(0));
        assert_eq!(mapper.get_numeric("second"), Some(1));
        assert_eq!(mapper.get_string(0), Some("first"));
        assert_eq!(mapper.get_string(1), Some("second"));
    }
}
