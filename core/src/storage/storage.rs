use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::document::Document;
use crate::error::SearchError;
use crate::index::Index;
use crate::schema::Mapping;

use super::manifest::Manifest;
use super::merge_policy::MergePolicy;
use super::segment_manager::SegmentManager;
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
    /// Segment lifecycle manager.
    segment_manager: SegmentManager,
    /// Merge policy that decides when to compact.
    merge_policy: MergePolicy,
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

        let index_names: Vec<String> = manifest.indexes.iter().map(|m| m.name.clone()).collect();
        let segment_manager = SegmentManager::open(&base_path, &index_names)?;

        // Step 2: Open WAL (create if missing, treat open errors as empty)
        let wal_path = base_path.join(Self::WAL_FILENAME);
        let wal = open_wal_safe(&wal_path)?;

        // Step 3: Load Segments (merge all active segments, fall back to snapshot)
        let indexes_dir = base_path.join(Self::INDEXES_DIR);
        let mut indexes = HashMap::new();

        for meta in &manifest.indexes {
            let index_dir = indexes_dir.join(&meta.name);
            if let Some(index) = segment_manager
                .load_merged_index(&meta.name)
                .map_err(|e| io::Error::other(e.to_string()))?
            {
                indexes.insert(meta.name.clone(), index);
            } else if let Some(snapshot) = Snapshot::load(&index_dir)? {
                indexes.insert(meta.name.clone(), snapshot.restore_index());
            }
        }

        // Step 4: Replay WAL entries
        let entries = wal.replay()?;
        let replay_failures = apply_entries(&mut indexes, &entries);
        if replay_failures > 0 {
            eprintln!(
                "WAL replay completed with {replay_failures} failed entr(ies) out of {} total",
                entries.len()
            );
        }

        // Step 5: Rebuild Statistics for consistency
        for index in indexes.values_mut() {
            index.rebuild_stats();
        }

        Ok(Self {
            base_path,
            indexes,
            wal,
            manifest,
            segment_manager,
            merge_policy: MergePolicy::default(),
        })
    }

    /// Create a new named index with the given mapping.
    ///
    /// The operation is written to the WAL before applying to in-memory state.
    pub fn create_index(&mut self, name: &str, mapping: Mapping) -> Result<(), SearchError> {
        if self.indexes.contains_key(name) {
            return Err(SearchError::IndexAlreadyExists(name.to_string()));
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
        self.segment_manager.ensure_index(name);
        self.manifest.upsert_index(name, vec![]);

        self.save_manifest().map_err(|e| internal_err(e))?;
        Ok(())
    }

    /// Delete an index and all its data.
    pub fn delete_index(&mut self, name: &str) -> Result<(), SearchError> {
        if !self.indexes.contains_key(name) {
            return Err(SearchError::IndexNotFound(name.to_string()));
        }

        self.wal
            .append(&WalEntry::DeleteIndex {
                name: name.to_string(),
            })
            .map_err(|e| internal_err(e))?;
        self.wal.flush().map_err(|e| internal_err(e))?;

        self.indexes.remove(name);
        self.manifest.remove_index(name);
        self.segment_manager.remove_index(name);

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
            .ok_or_else(|| SearchError::IndexNotFound(index_name.to_string()))?;

        index.add_document(document)?;
        Ok(())
    }

    /// Add a document without flushing the WAL (for batch operations).
    ///
    /// Caller MUST call `flush_wal()` after the batch to guarantee durability.
    pub fn add_document_no_flush(
        &mut self,
        index_name: &str,
        document: Document,
    ) -> Result<(), SearchError> {
        self.wal
            .append(&WalEntry::AddDocument {
                index_name: index_name.to_string(),
                document: document.clone(),
            })
            .map_err(|e| internal_err(e))?;

        let index = self
            .indexes
            .get_mut(index_name)
            .ok_or_else(|| SearchError::IndexNotFound(index_name.to_string()))?;

        index.add_document(document)?;
        Ok(())
    }

    /// Flush the write-ahead log to disk.
    ///
    /// Called after a batch of `add_document_no_flush` calls to guarantee
    /// durability for the entire batch with a single fsync.
    pub fn flush_wal(&mut self) -> Result<(), SearchError> {
        self.wal.flush().map_err(|e| internal_err(e))
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
            .ok_or_else(|| SearchError::IndexNotFound(index_name.to_string()))?;

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

        super::compaction::compact_with_manifest(
            &index_dir,
            name,
            &entries,
            &mut self.manifest,
        )
        .map_err(|e| internal_err(e))?;

        // Refresh segment registry from disk after compaction
        {
            let registry = self.segment_manager.ensure_index(name);
            registry
                .reconcile_with_disk(&index_dir)
                .map_err(|e| internal_err(e))?;
            registry
                .save(&index_dir)
                .map_err(|e| internal_err(e))?;
        }

        self.wal.truncate().map_err(|e| internal_err(e))?;
        self.segment_manager.sync_to_manifest(&mut self.manifest);
        self.save_manifest().map_err(|e| internal_err(e))?;
        Ok(())
    }

    /// Access the segment manager (read-only).
    pub fn segment_manager(&self) -> &SegmentManager {
        &self.segment_manager
    }

    /// Access the merge policy (read-only).
    pub fn merge_policy(&self) -> &MergePolicy {
        &self.merge_policy
    }

    /// Set a custom merge policy.
    pub fn set_merge_policy(&mut self, policy: MergePolicy) {
        self.merge_policy = policy;
    }

    /// Check all indexes and compact those that exceed the merge policy.
    ///
    /// This is the main entry point for automatic background compaction.
    /// It checks each index against the merge policy and triggers compaction
    /// for indexes that need it.
    ///
    /// Returns the list of index names that were compacted.
    pub fn compact_if_needed(&mut self) -> Result<Vec<String>, SearchError> {
        // First reconcile segment registries with what's on disk
        let index_names: Vec<String> = self.indexes.keys().cloned().collect();
        for name in &index_names {
            let index_dir = self.indexes_dir().join(name);
            if index_dir.exists() {
                let registry = self.segment_manager.ensure_index(name);
                registry
                    .reconcile_with_disk(&index_dir)
                    .map_err(|e| internal_err(e))?;
            }
        }

        let mut compacted = Vec::new();

        for name in &index_names {
            let segments = self.segment_manager.list_segments(name);
            if self.merge_policy.should_compact(&segments) {
                self.compact(name)?;
                compacted.push(name.clone());
            }
        }

        Ok(compacted)
    }

    /// Get the merge policy decision for an index.
    ///
    /// Returns which segments the policy recommends merging, or `None`
    /// if no merge is needed.
    pub fn merge_decision(
        &mut self,
        name: &str,
    ) -> Option<super::SegmentToMerge> {
        // Reconcile with disk first
        let index_dir = self.indexes_dir().join(name);
        if index_dir.exists() {
            if let Ok(()) = self
                .segment_manager
                .ensure_index(name)
                .reconcile_with_disk(&index_dir)
            {}
        }

        let segments = self.segment_manager.list_segments(name);
        let decision = self.merge_policy.select_segments(&segments);
        if matches!(decision, super::SegmentToMerge::None) {
            None
        } else {
            Some(decision)
        }
    }

    /// Get a reference to an index.
    pub fn get_index(&self, name: &str) -> Result<&Index, SearchError> {
        self.indexes
            .get(name)
            .ok_or_else(|| SearchError::IndexNotFound(name.to_string()))
    }

    /// Get a mutable reference to an index.
    pub fn get_index_mut(&mut self, name: &str) -> Result<&mut Index, SearchError> {
        self.indexes
            .get_mut(name)
            .ok_or_else(|| SearchError::IndexNotFound(name.to_string()))
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
///
/// Failed entries are logged and counted but do not halt recovery.
/// Returns the number of entries that failed to apply.
fn apply_entries(indexes: &mut HashMap<String, Index>, entries: &[WalEntry]) -> u64 {
    let mut failures = 0u64;
    for entry in entries {
        if let Err(e) = apply_single_entry(indexes, entry) {
            eprintln!("WAL replay: skipping entry — {e}");
            failures += 1;
        }
    }
    failures
}

/// Apply a single WAL entry, returning an error if it cannot be applied.
fn apply_single_entry(
    indexes: &mut HashMap<String, Index>,
    entry: &WalEntry,
) -> Result<(), String> {
    match entry {
        WalEntry::CreateIndex { name, mapping } => {
            if !indexes.contains_key(name) {
                let index = Index::new(name, mapping.clone());
                indexes.insert(name.clone(), index);
            }
            Ok(())
        }
        WalEntry::DeleteIndex { name } => {
            indexes.remove(name);
            Ok(())
        }
        WalEntry::AddDocument {
            index_name,
            document,
        } => {
            let index = indexes.get_mut(index_name).ok_or_else(|| {
                format!("index '{index_name}' not found for document '{}'", document.id)
            })?;
            index.add_document(document.clone()).map_err(|e| {
                format!("failed to add document '{}' to index '{index_name}': {e}", document.id)
            })?;
            Ok(())
        }
        WalEntry::RemoveDocument { index_name, doc_id } => {
            let index = indexes.get_mut(index_name).ok_or_else(|| {
                format!("index '{index_name}' not found for document removal '{doc_id}'")
            })?;
            index.remove_document(doc_id).map_err(|e| {
                format!("failed to remove document '{doc_id}' from index '{index_name}': {e}")
            })?;
            Ok(())
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;

    use crate::document::Document;
    use crate::storage::segment::{self, Segment};

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
    fn recovery_merges_multiple_segments() {
        let dir = tempfile::tempdir().unwrap();
        let index_dir = dir.path().join("indexes").join("multi");

        // Write two segments with different documents (simulating rolling flush)
        {
            fs::create_dir_all(&index_dir).unwrap();

            let mut manifest = Manifest::new();
            manifest.upsert_index("multi", vec![1, 2]);
            manifest.save(dir.path().join("manifest.json")).unwrap();

            let make_data =
                |ids: &[&str]| -> segment::IndexSegmentData {
                    let mut documents = HashMap::new();
                    for id in ids {
                        let mut fields = HashMap::new();
                        fields.insert("title".to_string(), serde_json::json!(id));
                        let doc = Document::new(*id, fields).unwrap();
                        documents.insert(id.to_string(), doc);
                    }
                    segment::IndexSegmentData {
                        name: "multi".into(),
                        mapping: Mapping::new(vec![]),
                        documents,
                        inverted_index: crate::index::inverted::InvertedIndex::new(),
                        stats: crate::ranking::statistics::CollectionStats::new(),
                    }
                };

            Segment::write_segment(&index_dir, 1, &make_data(&["doc1", "doc2"])).unwrap();
            Segment::write_segment(&index_dir, 2, &make_data(&["doc3"])).unwrap();
        }

        {
            let storage = setup_storage(dir.path());
            let index = storage.get_index("multi").unwrap();
            assert!(index.get_document("doc1").is_ok());
            assert!(index.get_document("doc2").is_ok());
            assert!(index.get_document("doc3").is_ok());

            let segments = storage.segment_manager().list_segments("multi");
            assert_eq!(segments.len(), 2);
            assert_eq!(storage.segment_manager().segment_ids("multi"), vec![1, 2]);
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
            // After rebuild_stats, search should work correctly
            let results = index.search("hello");
            assert_eq!(results.len(), 1, "search should work after stats rebuild");
        }
    }

    // --- compact_if_needed tests ---

    #[test]
    fn compact_if_needed_compacts_when_threshold_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());
        storage.create_index("test", Mapping::new(vec![])).unwrap();

        // Set a very low threshold to trigger compaction
        storage.set_merge_policy(MergePolicy::new(3, 1_000_000, 2, 5));

        // Write multiple segments directly
        let index_dir = dir.path().join("indexes").join("test");
        for i in 1..=5 {
            let mut documents = std::collections::HashMap::new();
            let mut fields = HashMap::new();
            fields.insert("title".to_string(), serde_json::json!("hello"));
            let doc = Document::new(&format!("doc{i}"), fields).unwrap();
            documents.insert(format!("doc{i}"), doc);

            let data = crate::storage::segment::IndexSegmentData {
                name: "test".to_string(),
                mapping: Mapping::new(vec![]),
                documents,
                inverted_index: crate::index::inverted::InvertedIndex::new(),
                stats: crate::ranking::statistics::CollectionStats::new(),
            };
            crate::storage::segment::Segment::write_segment(&index_dir, i, &data).unwrap();
        }

        // 5 segments > max_segment_count (3) — should compact
        let compacted = storage.compact_if_needed().unwrap();
        assert_eq!(compacted, vec!["test".to_string()]);

        // After compaction, should have 1 segment
        let segments = storage.segment_manager().list_segments("test");
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn compact_if_needed_skips_when_under_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());
        storage.create_index("test", Mapping::new(vec![])).unwrap();

        // Default policy: max_segment_count=10, so 2 segments won't trigger
        let index_dir = dir.path().join("indexes").join("test");
        let data = crate::storage::segment::IndexSegmentData {
            name: "test".to_string(),
            mapping: Mapping::new(vec![]),
            documents: std::collections::HashMap::new(),
            inverted_index: crate::index::inverted::InvertedIndex::new(),
            stats: crate::ranking::statistics::CollectionStats::new(),
        };
        crate::storage::segment::Segment::write_segment(&index_dir, 1, &data).unwrap();
        crate::storage::segment::Segment::write_segment(&index_dir, 2, &data).unwrap();

        let compacted = storage.compact_if_needed().unwrap();
        assert!(compacted.is_empty(), "should not compact when under threshold");
    }

    #[test]
    fn compact_if_needed_returns_compacted_index_names() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());
        storage.create_index("a", Mapping::new(vec![])).unwrap();
        storage.create_index("b", Mapping::new(vec![])).unwrap();

        // Set low threshold
        storage.set_merge_policy(MergePolicy::new(2, 1_000_000, 2, 5));

        // Write 3 segments to "a" (triggers compaction)
        let index_dir_a = dir.path().join("indexes").join("a");
        for i in 1..=3 {
            let data = crate::storage::segment::IndexSegmentData {
                name: "a".to_string(),
                mapping: Mapping::new(vec![]),
                documents: std::collections::HashMap::new(),
                inverted_index: crate::index::inverted::InvertedIndex::new(),
                stats: crate::ranking::statistics::CollectionStats::new(),
            };
            crate::storage::segment::Segment::write_segment(&index_dir_a, i, &data).unwrap();
        }

        // Write 1 segment to "b" (does NOT trigger)
        let index_dir_b = dir.path().join("indexes").join("b");
        let data = crate::storage::segment::IndexSegmentData {
            name: "b".to_string(),
            mapping: Mapping::new(vec![]),
            documents: std::collections::HashMap::new(),
            inverted_index: crate::index::inverted::InvertedIndex::new(),
            stats: crate::ranking::statistics::CollectionStats::new(),
        };
        crate::storage::segment::Segment::write_segment(&index_dir_b, 1, &data).unwrap();

        let compacted = storage.compact_if_needed().unwrap();
        assert_eq!(compacted, vec!["a".to_string()]);
    }

    #[test]
    fn merge_decision_returns_none_when_no_merge_needed() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());
        storage.create_index("test", Mapping::new(vec![])).unwrap();

        let decision = storage.merge_decision("test");
        assert!(decision.is_none());
    }

    #[test]
    fn merge_decision_returns_merge_when_needed() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());
        storage.create_index("test", Mapping::new(vec![])).unwrap();

        storage.set_merge_policy(MergePolicy::new(3, 1_000_000, 2, 5));

        let index_dir = dir.path().join("indexes").join("test");
        for i in 1..=5 {
            let data = crate::storage::segment::IndexSegmentData {
                name: "test".to_string(),
                mapping: Mapping::new(vec![]),
                documents: std::collections::HashMap::new(),
                inverted_index: crate::index::inverted::InvertedIndex::new(),
                stats: crate::ranking::statistics::CollectionStats::new(),
            };
            crate::storage::segment::Segment::write_segment(&index_dir, i, &data).unwrap();
        }

        let decision = storage.merge_decision("test");
        assert!(decision.is_some());
        match decision.unwrap() {
            crate::storage::SegmentToMerge::Merge(ids) => {
                assert!(ids.len() >= 2, "should merge at least 2 segments");
            }
            _ => panic!("expected Merge decision"),
        }
    }

    #[test]
    fn compact_preserves_search_after_compaction() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = setup_storage(dir.path());
        storage.create_index("test", Mapping::new(vec![])).unwrap();

        // Add documents through the normal API
        for (id, title) in [("doc1", "hello world"), ("doc2", "hello there"), ("doc3", "goodbye")] {
            let mut fields = HashMap::new();
            fields.insert("title".to_string(), serde_json::json!(title));
            let doc = Document::new(id, fields).unwrap();
            storage.add_document("test", doc).unwrap();
        }

        // Force compaction
        storage.compact("test").unwrap();

        // Search must still work after compaction
        let index = storage.get_index("test").unwrap();
        let results = index.search("hello");
        assert_eq!(results.len(), 2, "search must find both docs after compaction");
        let ids: Vec<&str> = results.iter().map(|r| r.document_id.as_str()).collect();
        assert!(ids.contains(&"doc1"));
        assert!(ids.contains(&"doc2"));
    }
}
