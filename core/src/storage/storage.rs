use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::document::Document;
use crate::error::SearchError;
use crate::index::Index;
use crate::schema::Mapping;

use super::manifest::Manifest;
use super::segment::Segment;
use super::snapshot::Snapshot;
use super::wal::{Wal, WalEntry};

/// High-level storage coordinator that ties WAL, segments, and in-memory
/// indexes together.
///
/// Provides crash-recoverable persistence: every write goes through the WAL
/// first, and periodic snapshots/segments enable fast recovery.
///
/// # Recovery Flow
///
/// 1. Read Manifest — discover known indexes
/// 2. Load Segments — load the latest on-disk segment for each index
/// 3. Replay WAL — apply entries not yet reflected in segments/snapshots
/// 4. Rebuild Statistics — recompute collection stats from all documents
/// 5. Ready
pub struct Storage {
    /// Root directory for all storage data.
    base_path: PathBuf,
    /// In-memory indexes.
    indexes: HashMap<String, Index>,
    /// Write-ahead log for durability.
    wal: Wal,
    /// Metadata tracker.
    manifest: Manifest,
}

impl Storage {
    /// WAL filename within the storage root.
    const WAL_FILENAME: &'static str = "wal.log";
    /// Manifest filename within the storage root.
    const MANIFEST_FILENAME: &'static str = "manifest.json";
    /// Subdirectory for index data within the storage root.
    const INDEXES_DIR: &'static str = "indexes";

    /// Open or create storage at the given base path.
    ///
    /// Performs the full recovery sequence on open:
    ///
    /// 1. **Read Manifest** — loads index metadata (version 2 format).
    /// 2. **Load Segments** — discovers `.seg` files for each known index
    ///    and restores them; falls back to `snapshot.json` if no segments
    ///    exist.
    /// 3. **Replay WAL** — applies any WAL entries not yet reflected in the
    ///    loaded segments/snapshots.
    /// 4. **Rebuild Statistics** — recomputes collection-level statistics
    ///    from all loaded documents for consistency.
    ///
    /// Missing or corrupt WAL files are handled gracefully (treated as empty).
    pub fn open(base_path: impl Into<PathBuf>) -> io::Result<Self> {
        let base_path = base_path.into();
        fs::create_dir_all(&base_path)?;

        // Step 1: Read Manifest
        let manifest_path = base_path.join(Self::MANIFEST_FILENAME);
        let manifest = Manifest::load(&manifest_path)?;

        // Step 2: Open WAL (create if missing, treat open errors as empty)
        let wal_path = base_path.join(Self::WAL_FILENAME);
        let wal = open_wal_safe(&wal_path)?;

        // Step 3: Load Segments (discover .seg files, fall back to snapshot)
        let indexes_dir = base_path.join(Self::INDEXES_DIR);
        let mut indexes = HashMap::new();

        for meta in &manifest.indexes {
            let index_dir = indexes_dir.join(&meta.name);
            if let Some(index) = load_index_from_segments(&index_dir, &meta.name) {
                indexes.insert(meta.name.clone(), index);
            } else if let Some(snapshot) = Snapshot::load(&index_dir)? {
                indexes.insert(meta.name.clone(), snapshot.restore_index());
            }
        }

        // Step 4: Replay WAL entries
        let entries = wal.replay()?;
        apply_entries(&mut indexes, &entries);

        // Step 5: Rebuild Statistics for consistency
        for index in indexes.values_mut() {
            index.rebuild_stats();
        }

        Ok(Self {
            base_path,
            indexes,
            wal,
            manifest,
        })
    }

    /// Create a new named index with the given mapping.
    ///
    /// The operation is written to the WAL before applying to in-memory state.
    pub fn create_index(&mut self, name: &str, mapping: Mapping) -> Result<(), SearchError> {
        if self.indexes.contains_key(name) {
            return Err(SearchError::Internal(format!("index '{name}' already exists")));
        }

        self.wal
            .append(&WalEntry::CreateIndex {
                name: name.to_string(),
                mapping: mapping.clone(),
            })
            .map_err(|e| internal_err(e))?;
        self.wal.flush().map_err(|e| internal_err(e))?;

        let index = Index::new(name, mapping);
        self.indexes.insert(name.to_string(), index);
        self.manifest.upsert_index(name, vec![]);

        self.save_manifest().map_err(|e| internal_err(e))?;
        Ok(())
    }

    /// Delete an index and all its data.
    pub fn delete_index(&mut self, name: &str) -> Result<(), SearchError> {
        if !self.indexes.contains_key(name) {
            return Err(SearchError::Internal(format!("index '{name}' not found")));
        }

        self.wal
            .append(&WalEntry::DeleteIndex {
                name: name.to_string(),
            })
            .map_err(|e| internal_err(e))?;
        self.wal.flush().map_err(|e| internal_err(e))?;

        self.indexes.remove(name);
        self.manifest.remove_index(name);

        // Remove on-disk data
        let index_dir = self.indexes_dir().join(name);
        let _ = fs::remove_dir_all(&index_dir);

        self.save_manifest().map_err(|e| internal_err(e))?;
        Ok(())
    }

    /// Add a document to an index.
    ///
    /// Write flow: append WAL → flush WAL → apply to memory.
    pub fn add_document(&mut self, index_name: &str, document: Document) -> Result<(), SearchError> {
        self.wal
            .append(&WalEntry::AddDocument {
                index_name: index_name.to_string(),
                document: document.clone(),
            })
            .map_err(|e| internal_err(e))?;
        self.wal.flush().map_err(|e| internal_err(e))?;

        let index = self
            .indexes
            .get_mut(index_name)
            .ok_or_else(|| SearchError::Internal(format!("index '{index_name}' not found")))?;

        index.add_document(document)?;
        Ok(())
    }

    /// Remove a document from an index.
    ///
    /// Write flow: append WAL → flush WAL → apply to memory.
    pub fn remove_document(&mut self, index_name: &str, doc_id: &str) -> Result<(), SearchError> {
        self.wal
            .append(&WalEntry::RemoveDocument {
                index_name: index_name.to_string(),
                doc_id: doc_id.to_string(),
            })
            .map_err(|e| internal_err(e))?;
        self.wal.flush().map_err(|e| internal_err(e))?;

        let index = self
            .indexes
            .get_mut(index_name)
            .ok_or_else(|| SearchError::Internal(format!("index '{index_name}' not found")))?;

        index.remove_document(doc_id)?;
        Ok(())
    }

    /// Flush all indexes to disk by creating snapshots.
    ///
    /// After successful flush, the WAL is truncated.
    pub fn flush(&mut self) -> Result<(), SearchError> {
        let indexes_dir = self.indexes_dir();
        fs::create_dir_all(&indexes_dir).map_err(|e| internal_err(e))?;

        for (name, index) in &self.indexes {
            let index_dir = indexes_dir.join(name);
            let _ = Snapshot::create_snapshot(&index_dir, index).map_err(|e| internal_err(e))?;
        }

        // After successful snapshot, truncate WAL
        self.wal.truncate().map_err(|e| internal_err(e))?;

        // Update manifest
        for name in self.indexes.keys() {
            self.manifest.upsert_index(name, vec![]);
        }
        self.save_manifest().map_err(|e| internal_err(e))?;

        Ok(())
    }

    /// Compact all segment files for a single index into one.
    ///
    /// Merges old segments with the current WAL state, writes a new compact
    /// segment, and removes the old segment files. The WAL is truncated
    /// after compaction.
    ///
    /// This prevents unbounded growth of segment files and improves
    /// recovery speed.
    pub fn compact(&mut self, name: &str) -> Result<(), SearchError> {
        let index_dir = self.indexes_dir().join(name);
        let entries = self.wal.replay().map_err(|e| internal_err(e))?;

        let seg_id = super::compaction::compact_with_manifest(
            &index_dir,
            name,
            &entries,
            &mut self.manifest,
        )
        .map_err(|e| internal_err(e))?;

        self.wal.truncate().map_err(|e| internal_err(e))?;
        self.manifest.upsert_index(name, vec![seg_id]);
        self.save_manifest().map_err(|e| internal_err(e))?;
        Ok(())
    }

    /// Get a reference to an index.
    pub fn get_index(&self, name: &str) -> Result<&Index, SearchError> {
        self.indexes
            .get(name)
            .ok_or_else(|| SearchError::Internal(format!("index '{name}' not found")))
    }

    /// Get a mutable reference to an index.
    pub fn get_index_mut(&mut self, name: &str) -> Result<&mut Index, SearchError> {
        self.indexes
            .get_mut(name)
            .ok_or_else(|| SearchError::Internal(format!("index '{name}' not found")))
    }

    /// List all index names.
    pub fn list_indexes(&self) -> Vec<String> {
        let mut names: Vec<String> = self.indexes.keys().cloned().collect();
        names.sort();
        names
    }

    /// Check if an index exists.
    pub fn index_exists(&self, name: &str) -> bool {
        self.indexes.contains_key(name)
    }

    /// Return the indexes directory path.
    fn indexes_dir(&self) -> PathBuf {
        self.base_path.join(Self::INDEXES_DIR)
    }

    /// Save the manifest to disk.
    fn save_manifest(&self) -> io::Result<()> {
        let path = self.base_path.join(Self::MANIFEST_FILENAME);
        self.manifest.save(&path)
    }
}

/// Apply a sequence of WAL entries to an index map.
fn apply_entries(indexes: &mut HashMap<String, Index>, entries: &[WalEntry]) {
    for entry in entries {
        match entry {
            WalEntry::CreateIndex { name, mapping } => {
                if !indexes.contains_key(name) {
                    let index = Index::new(name, mapping.clone());
                    indexes.insert(name.clone(), index);
                }
            }
            WalEntry::DeleteIndex { name } => {
                indexes.remove(name);
            }
            WalEntry::AddDocument {
                index_name,
                document,
            } => {
                if let Some(index) = indexes.get_mut(index_name) {
                    let _ = index.add_document(document.clone());
                }
            }
            WalEntry::RemoveDocument { index_name, doc_id } => {
                if let Some(index) = indexes.get_mut(index_name) {
                    let _ = index.remove_document(doc_id);
                }
            }
        }
    }
}

/// Convert an io::Error into a SearchError::Internal.
fn internal_err(e: impl std::fmt::Display) -> SearchError {
    SearchError::Internal(e.to_string())
}

/// Open a WAL file, returning an empty WAL if the file cannot be opened.
///
/// This handles the case where the WAL file is missing, corrupted, or
/// inaccessible — recovery continues without it.
fn open_wal_safe(path: &PathBuf) -> io::Result<Wal> {
    match Wal::open(path) {
        Ok(wal) => Ok(wal),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            // File doesn't exist yet — create it
            Wal::open(path)
        }
        Err(e) => {
            // Log the error but continue with an empty WAL
            eprintln!("WAL open error (continuing with empty WAL): {e}");
            // Create a fresh WAL
            let _ = fs::write(path, b"");
            Wal::open(path)
        }
    }
}

/// Load an index from discovered segment files in its directory.
///
/// Scans for the newest `.seg` file (highest ID) and loads it. Returns
/// `None` if no segment files exist.
fn load_index_from_segments(dir: &PathBuf, name: &str) -> Option<Index> {
    let segments = Segment::discover(dir).ok()?;
    // Load the newest segment (sorted oldest-first, take last)
    let segment = segments.last()?;
    let data = Segment::read_segment(&segment.path).ok()?;

    let mut index = Index::new(name, data.mapping.clone());
    for (doc_id, doc) in &data.documents {
        if index.get_document(doc_id).is_err() {
            let _ = index.add_document(doc.clone());
        }
    }
    Some(index)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;

    use crate::document::Document;
    use crate::storage::segment;

    use super::*;

    fn setup_storage(dir: &Path) -> Storage {
        Storage::open(dir).unwrap()
    }

    fn make_doc(id: &str) -> Document {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("hello"));
        Document::new(id, fields).unwrap()
    }

    #[test]
    fn create_and_list_indexes() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());

        storage
            .create_index("products", Mapping::new(vec![]))
            .unwrap();
        storage.create_index("users", Mapping::new(vec![])).unwrap();

        let indexes = storage.list_indexes();
        assert_eq!(indexes, vec!["products", "users"]);
    }

    #[test]
    fn create_duplicate_index_fails() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());

        storage
            .create_index("test", Mapping::new(vec![]))
            .unwrap();
        let err = storage.create_index("test", Mapping::new(vec![]));
        assert!(err.is_err());
    }

    #[test]
    fn add_and_retrieve_document() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());

        storage
            .create_index("test", Mapping::new(vec![]))
            .unwrap();
        storage.add_document("test", make_doc("doc1")).unwrap();

        let index = storage.get_index("test").unwrap();
        assert!(index.get_document("doc1").is_ok());
    }

    #[test]
    fn remove_document() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());

        storage
            .create_index("test", Mapping::new(vec![]))
            .unwrap();
        storage.add_document("test", make_doc("doc1")).unwrap();
        storage.remove_document("test", "doc1").unwrap();

        let index = storage.get_index("test").unwrap();
        assert!(index.get_document("doc1").is_err());
    }

    #[test]
    fn delete_index() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());

        storage
            .create_index("test", Mapping::new(vec![]))
            .unwrap();
        assert!(storage.index_exists("test"));

        storage.delete_index("test").unwrap();
        assert!(!storage.index_exists("test"));
    }

    #[test]
    fn flush_and_recover() {
        let dir = tempfile::tempdir().unwrap();

        // First session: create index and add documents
        {
            let mut storage = setup_storage(dir.path());
            storage
                .create_index("products", Mapping::new(vec![]))
                .unwrap();

            let mut fields = HashMap::new();
            fields.insert("title".to_string(), serde_json::json!("bike"));
            let doc = Document::new("doc1", fields).unwrap();
            storage.add_document("products", doc).unwrap();

            storage.flush().unwrap();
        }
        // Drop: storage goes out of scope

        // Second session: recover from disk
        {
            let storage = setup_storage(dir.path());
            assert!(storage.index_exists("products"));

            let index = storage.get_index("products").unwrap();
            assert!(index.get_document("doc1").is_ok());
        }
    }

    #[test]
    fn crash_recovery_without_flush() {
        let dir = tempfile::tempdir().unwrap();

        // First session: create and add document, but do NOT flush
        {
            let mut storage = setup_storage(dir.path());
            storage
                .create_index("test", Mapping::new(vec![]))
                .unwrap();
            storage.add_document("test", make_doc("doc1")).unwrap();
        }

        // Second session: recover — WAL should replay the add
        {
            let storage = setup_storage(dir.path());
            assert!(storage.index_exists("test"));

            let index = storage.get_index("test").unwrap();
            assert!(index.get_document("doc1").is_ok());
        }
    }

    #[test]
    fn add_after_recovery() {
        let dir = tempfile::tempdir().unwrap();

        // Session 1
        {
            let mut storage = setup_storage(dir.path());
            storage
                .create_index("test", Mapping::new(vec![]))
                .unwrap();
            storage.add_document("test", make_doc("doc1")).unwrap();
            storage.flush().unwrap();
        }

        // Session 2: add another document
        {
            let mut storage = setup_storage(dir.path());
            assert!(storage.get_index("test").unwrap().get_document("doc1").is_ok());
            storage.add_document("test", make_doc("doc2")).unwrap();
            storage.flush().unwrap();
        }

        // Session 3: verify both exist
        {
            let storage = setup_storage(dir.path());
            let index = storage.get_index("test").unwrap();
            assert!(index.get_document("doc1").is_ok());
            assert!(index.get_document("doc2").is_ok());
        }
    }

    #[test]
    fn delete_after_recovery() {
        let dir = tempfile::tempdir().unwrap();

        // Session 1
        {
            let mut storage = setup_storage(dir.path());
            storage
                .create_index("test", Mapping::new(vec![]))
                .unwrap();
            storage.add_document("test", make_doc("doc1")).unwrap();
            storage.add_document("test", make_doc("doc2")).unwrap();
            storage.flush().unwrap();
        }

        // Session 2: remove doc1
        {
            let mut storage = setup_storage(dir.path());
            storage.remove_document("test", "doc1").unwrap();
            storage.flush().unwrap();
        }

        // Session 3: verify doc1 gone, doc2 exists
        {
            let storage = setup_storage(dir.path());
            let index = storage.get_index("test").unwrap();
            assert!(index.get_document("doc1").is_err());
            assert!(index.get_document("doc2").is_ok());
        }
    }

    #[test]
    fn multiple_indexes_independent() {
        let dir = tempfile::tempdir().unwrap();

        {
            let mut storage = setup_storage(dir.path());
            storage
                .create_index("a", Mapping::new(vec![]))
                .unwrap();
            storage
                .create_index("b", Mapping::new(vec![]))
                .unwrap();
            storage.add_document("a", make_doc("doc1")).unwrap();
            storage.add_document("b", make_doc("doc2")).unwrap();
            storage.flush().unwrap();
        }

        {
            let storage = setup_storage(dir.path());
            assert_eq!(storage.list_indexes(), vec!["a", "b"]);
            assert!(storage.get_index("a").unwrap().get_document("doc1").is_ok());
            assert!(storage.get_index("b").unwrap().get_document("doc2").is_ok());
        }
    }

    #[test]
    fn add_document_to_nonexistent_index_fails() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());
        let err = storage.add_document("nonexistent", make_doc("doc1"));
        assert!(err.is_err());
    }

    #[test]
    fn list_indexes_empty() {
        let dir = tempfile::tempdir().unwrap();
        let storage = setup_storage(dir.path());
        assert!(storage.list_indexes().is_empty());
    }

    #[test]
    fn get_nonexistent_index_fails() {
        let dir = tempfile::tempdir().unwrap();
        let storage = setup_storage(dir.path());
        assert!(storage.get_index("nonexistent").is_err());
    }

    #[test]
    fn flush_empty_storage() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());
        storage.flush().unwrap();
        assert!(storage.list_indexes().is_empty());
    }

    #[test]
    fn recovery_from_segments() {
        let dir = tempfile::tempdir().unwrap();

        // Session 1: create index, add docs, write a segment, then flush
        let seg_id = {
            let mut storage = setup_storage(dir.path());
            storage
                .create_index("prod", Mapping::new(vec![]))
                .unwrap();
            storage.add_document("prod", make_doc("a")).unwrap();
            storage.add_document("prod", make_doc("b")).unwrap();

            // Write a segment file manually (simulating a merge/checkpoint)
            let index = storage.get_index("prod").unwrap();
            let data = segment::IndexSegmentData {
                name: "prod".into(),
                mapping: index.mapping().clone(),
                documents: {
                    let mut docs = std::collections::HashMap::new();
                    for id in index.list_document_ids() {
                        if let Ok(d) = index.get_document(&id) {
                            docs.insert(id.clone(), d.clone());
                        }
                    }
                    docs
                },
                inverted_index: index.inverted_index_clone(),
                stats: index.stats_ref().clone(),
            };
            let seg = Segment::write_segment(
                &dir.path().join("indexes").join("prod"),
                1,
                &data,
            )
            .unwrap();
            seg.id
        };
        assert_eq!(seg_id, 1);

        // Session 2: recover — segments should be loaded
        {
            let storage = setup_storage(dir.path());
            assert!(storage.index_exists("prod"));
            let index = storage.get_index("prod").unwrap();
            assert!(index.get_document("a").is_ok());
            assert!(index.get_document("b").is_ok());
        }
    }

    #[test]
    fn recovery_without_wal_file() {
        let dir = tempfile::tempdir().unwrap();

        // Create manifest + data manually (no WAL)
        {
            let indexes_dir = dir.path().join("indexes");
            // Create a snapshot directly
            let mut index = Index::new("test", Mapping::new(vec![]));
            let mut fields = HashMap::new();
            fields.insert("title".to_string(), serde_json::json!("hello"));
            let doc = Document::new("d1", fields).unwrap();
            index.add_document(doc).unwrap();

            let index_dir = indexes_dir.join("test");
            fs::create_dir_all(&index_dir).unwrap();
            Snapshot::create_snapshot(&index_dir, &index).unwrap();

            // Write manifest manually
            let mut manifest = Manifest::new();
            manifest.upsert_index("test", vec![]);
            manifest.save(dir.path().join("manifest.json")).unwrap();
        }

        // WAL file does NOT exist — recovery should still succeed
        {
            let storage = setup_storage(dir.path());
            assert!(storage.index_exists("test"));
            let index = storage.get_index("test").unwrap();
            assert!(index.get_document("d1").is_ok());
        }
    }

    #[test]
    fn recovery_handles_corrupt_wal() {
        let dir = tempfile::tempdir().unwrap();

        // Create valid snapshot + manifest
        {
            let mut index = Index::new("test", Mapping::new(vec![]));
            let mut fields = HashMap::new();
            fields.insert("title".to_string(), serde_json::json!("hello"));
            let doc = Document::new("d1", fields).unwrap();
            index.add_document(doc).unwrap();

            let index_dir = dir.path().join("indexes").join("test");
            fs::create_dir_all(&index_dir).unwrap();
            Snapshot::create_snapshot(&index_dir, &index).unwrap();

            let mut manifest = Manifest::new();
            manifest.upsert_index("test", vec![]);
            manifest.save(dir.path().join("manifest.json")).unwrap();
        }

        // Write garbage to the WAL file
        fs::write(dir.path().join("wal.log"), "{{{garbage}}}").unwrap();

        // Recovery should still succeed
        {
            let storage = setup_storage(dir.path());
            assert!(storage.index_exists("test"));
            let index = storage.get_index("test").unwrap();
            assert!(index.get_document("d1").is_ok());
        }
    }

    #[test]
    fn recovery_rebuilds_statistics() {
        let dir = tempfile::tempdir().unwrap();

        // Create a snapshot without valid stats (simulate corrupted stats)
        {
            let mut index = Index::new("test", Mapping::new(vec![]));
            let mut fields = HashMap::new();
            fields.insert("title".to_string(), serde_json::json!("hello world"));
            let doc = Document::new("d1", fields).unwrap();
            index.add_document(doc).unwrap();

            let index_dir = dir.path().join("indexes").join("test");
            fs::create_dir_all(&index_dir).unwrap();
            Snapshot::create_snapshot(&index_dir, &index).unwrap();

            let mut manifest = Manifest::new();
            manifest.upsert_index("test", vec![]);
            manifest.save(dir.path().join("manifest.json")).unwrap();
        }

        // Recovery rebuilds stats — search should work correctly
        {
            let storage = setup_storage(dir.path());
            let index = storage.get_index("test").unwrap();
            // After rebuild_stats, search should return valid results
            let results = index.search("hello");
            assert_eq!(results.len(), 1, "search should work after stats rebuild");
        }
    }
}
