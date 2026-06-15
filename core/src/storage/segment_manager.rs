use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::index::Index;

use super::manifest::Manifest;
use super::segment::{IndexSegmentData, Segment};
use super::segment_metadata::{SegmentMetadata, SegmentState};
use super::segment_registry::SegmentRegistry;

/// Coordinates segment lifecycle across all indexes in a storage root.
///
/// Each index maintains its own [`SegmentRegistry`] tracking segment IDs,
/// document counts, creation times, and file sizes. Registries are persisted
/// to `{base}/indexes/{name}/segments.json`.
pub struct SegmentManager {
    base_path: PathBuf,
    registries: HashMap<String, SegmentRegistry>,
}

impl SegmentManager {
    /// Subdirectory for index data within the storage root.
    const INDEXES_DIR: &'static str = "indexes";

    /// Open or initialize segment managers for all known indexes.
    pub fn open(base_path: impl Into<PathBuf>, index_names: &[String]) -> io::Result<Self> {
        let base_path = base_path.into();
        let indexes_dir = base_path.join(Self::INDEXES_DIR);
        let mut registries = HashMap::new();

        for name in index_names {
            let index_dir = indexes_dir.join(name);
            let registry = if index_dir.exists() {
                SegmentRegistry::load(&index_dir, name)?
            } else {
                SegmentRegistry::new(name)
            };
            registries.insert(name.clone(), registry);
        }

        // Discover indexes that exist on disk but aren't in the manifest yet
        if indexes_dir.exists() {
            for entry in fs::read_dir(&indexes_dir)? {
                let entry = entry?;
                if !entry.file_type()?.is_dir() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if registries.contains_key(&name) {
                    continue;
                }
                let registry = SegmentRegistry::load(&entry.path(), &name)?;
                registries.insert(name, registry);
            }
        }

        Ok(Self {
            base_path,
            registries,
        })
    }

    /// Ensure a registry exists for the given index name.
    pub fn ensure_index(&mut self, index_name: &str) -> &mut SegmentRegistry {
        self.registries
            .entry(index_name.to_string())
            .or_insert_with(|| SegmentRegistry::new(index_name))
    }

    /// Remove all segment tracking for a deleted index.
    pub fn remove_index(&mut self, index_name: &str) {
        self.registries.remove(index_name);
    }

    /// Read-only access to an index registry.
    pub fn registry(&self, index_name: &str) -> Option<&SegmentRegistry> {
        self.registries.get(index_name)
    }

    /// List segment metadata for an index (active segments, sorted by ID).
    pub fn list_segments(&self, index_name: &str) -> Vec<SegmentMetadata> {
        self.registry(index_name)
            .map(|r| r.active_segments().into_iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Sorted active segment IDs for an index.
    pub fn segment_ids(&self, index_name: &str) -> Vec<u64> {
        self.registry(index_name)
            .map(|r| r.active_segment_ids())
            .unwrap_or_default()
    }

    /// Write a new segment with an auto-allocated ID and register its metadata.
    pub fn write_segment(
        &mut self,
        index_name: &str,
        data: &IndexSegmentData,
    ) -> io::Result<SegmentMetadata> {
        let index_dir = self.index_dir(index_name);
        fs::create_dir_all(&index_dir)?;

        let registry = self.ensure_index(index_name);
        let id = registry.next_segment_id();
        self.write_segment_with_id(index_name, id, data)
    }

    /// Write a segment with a specific ID and register its metadata.
    pub fn write_segment_with_id(
        &mut self,
        index_name: &str,
        id: u64,
        data: &IndexSegmentData,
    ) -> io::Result<SegmentMetadata> {
        let index_dir = self.index_dir(index_name);
        fs::create_dir_all(&index_dir)?;

        let seg = Segment::write_segment(&index_dir, id, data)?;
        let size_bytes = fs::metadata(&seg.path)?.len();
        let meta = SegmentMetadata::from_segment_data(id, size_bytes, data);

        let registry = self.ensure_index(index_name);
        registry.register(meta.clone());
        registry.save(&index_dir)?;

        Ok(meta)
    }

    /// Mark a segment deleted, remove its file, and persist the registry.
    pub fn delete_segment(&mut self, index_name: &str, id: u64) -> io::Result<()> {
        let index_dir = self.index_dir(index_name);
        let path = SegmentMetadata::segment_path(&index_dir, id);
        Segment::delete_segment(&path)?;

        if let Some(registry) = self.registries.get_mut(index_name) {
            registry.remove(id);
            registry.save(&index_dir)?;
        }
        Ok(())
    }

    /// Replace all segments for an index with a single compact segment.
    ///
    /// Deletes old segment files, registers the new segment, and persists
    /// metadata. Returns the new segment's metadata.
    pub fn replace_with_compact_segment(
        &mut self,
        index_name: &str,
        data: &IndexSegmentData,
        old_ids: &[u64],
        new_id: u64,
    ) -> io::Result<SegmentMetadata> {
        let index_dir = self.index_dir(index_name);
        fs::create_dir_all(&index_dir)?;

        // Delete old segments FIRST to avoid immutability collisions.
        // The new_id must not collide with any old_id (caller ensures this).
        for &old_id in old_ids {
            if old_id != new_id {
                let path = SegmentMetadata::segment_path(&index_dir, old_id);
                Segment::delete_segment(&path)?;
            }
        }

        // Write the compact segment atomically. If new_id already exists
        // (e.g., from a previous partial compaction), delete it first.
        let new_path = SegmentMetadata::segment_path(&index_dir, new_id);
        if new_path.exists() {
            Segment::delete_segment(&new_path)?;
        }

        let seg = Segment::write_segment(&index_dir, new_id, data)?;
        let size_bytes = fs::metadata(&seg.path)?.len();
        let meta = SegmentMetadata::from_segment_data(new_id, size_bytes, data);

        let registry = self.ensure_index(index_name);
        registry.segments.clear();
        registry.register(meta.clone());
        registry.save(&index_dir)?;

        Ok(meta)
    }

    /// Load and merge all active segments for an index into a single in-memory
    /// [`Index`].
    ///
    /// Segments are processed in ascending ID order. When the same document
    /// ID appears in multiple segments, the first occurrence wins (deterministic,
    /// oldest segment takes precedence).
    pub fn load_merged_index(&self, index_name: &str) -> io::Result<Option<Index>> {
        let index_dir = self.index_dir(index_name);

        // Build segment list: prefer registry, fall back to disk discovery
        let segment_ids: Vec<u64> = if let Some(registry) = self.registry(index_name) {
            registry.active_segment_ids()
        } else {
            Segment::discover(&index_dir)?
                .into_iter()
                .map(|s| s.id)
                .collect()
        };

        if segment_ids.is_empty() {
            return Ok(None);
        }

        let mut index: Option<Index> = None;

        for id in segment_ids {
            let path = SegmentMetadata::segment_path(&index_dir, id);
            if !path.exists() {
                continue;
            }
            let data = Segment::read_segment(&path)?;

            if index.is_none() {
                index = Some(Index::new(index_name, data.mapping.clone()));
            }

            if let Some(ref mut idx) = index {
                for (_, doc) in &data.documents {
                    if idx.get_document(&doc.id).is_err() {
                        let _ = idx.add_document(doc.clone());
                    }
                }
            }
        }

        Ok(index)
    }

    /// Persist all registries to disk.
    pub fn save_all(&self) -> io::Result<()> {
        for (name, registry) in &self.registries {
            let index_dir = self.index_dir(name);
            registry.save(&index_dir)?;
        }
        Ok(())
    }

    /// Sync active segment IDs into the root manifest (backward compatible).
    pub fn sync_to_manifest(&self, manifest: &mut Manifest) {
        for (name, registry) in &self.registries {
            manifest.upsert_index(name, registry.active_segment_ids());
        }
    }

    /// Mark segments as merging (used before background compaction).
    pub fn mark_merging(&mut self, index_name: &str, ids: &[u64]) -> io::Result<()> {
        let index_dir = self.index_dir(index_name);
        if let Some(registry) = self.registries.get_mut(index_name) {
            for &id in ids {
                if let Some(meta) = registry.segments.get_mut(&id) {
                    meta.state = SegmentState::Merging;
                }
            }
            registry.save(&index_dir)?;
        }
        Ok(())
    }

    fn index_dir(&self, index_name: &str) -> PathBuf {
        self.base_path
            .join(Self::INDEXES_DIR)
            .join(index_name)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::index::inverted::InvertedIndex;
    use crate::ranking::statistics::CollectionStats;
    use crate::schema::Mapping;
    use crate::storage::manifest::Manifest;
    use crate::storage::segment::IndexSegmentData;

    use super::*;

    fn make_data(name: &str, doc_ids: &[&str]) -> IndexSegmentData {
        let mut documents = HashMap::new();
        for id in doc_ids {
            let mut fields = HashMap::new();
            fields.insert("title".to_string(), serde_json::json!(id));
            let doc = Document::new(*id, fields).unwrap();
            documents.insert(id.to_string(), doc);
        }
        IndexSegmentData {
            name: name.into(),
            mapping: Mapping::new(vec![]),
            documents,
            inverted_index: InvertedIndex::new(),
            stats: CollectionStats::new(),
        }
    }

    #[test]
    fn write_multiple_segments() {
        let dir = tempfile::tempdir().unwrap();
        let mut mgr = SegmentManager::open(dir.path(), &["products".into()]).unwrap();

        let m1 = mgr
            .write_segment("products", &make_data("products", &["a", "b"]))
            .unwrap();
        let m2 = mgr
            .write_segment("products", &make_data("products", &["c"]))
            .unwrap();

        assert_eq!(m1.id, 1);
        assert_eq!(m2.id, 2);
        assert_eq!(m1.document_count, 2);
        assert_eq!(m2.document_count, 1);

        let segments = mgr.list_segments("products");
        assert_eq!(segments.len(), 2);
        assert!(segments[0].created_at <= segments[1].created_at);
    }

    #[test]
    fn load_merged_index_combines_segments() {
        let dir = tempfile::tempdir().unwrap();
        let mut mgr = SegmentManager::open(dir.path(), &["test".into()]).unwrap();

        mgr.write_segment("test", &make_data("test", &["doc1", "doc2"]))
            .unwrap();
        mgr.write_segment("test", &make_data("test", &["doc3"]))
            .unwrap();

        let index = mgr.load_merged_index("test").unwrap().unwrap();
        assert!(index.get_document("doc1").is_ok());
        assert!(index.get_document("doc2").is_ok());
        assert!(index.get_document("doc3").is_ok());
    }

    #[test]
    fn load_merged_index_first_wins_on_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        let mut mgr = SegmentManager::open(dir.path(), &["test".into()]).unwrap();

        mgr.write_segment("test", &make_data("test", &["dup"]))
            .unwrap();
        mgr.write_segment("test", &make_data("test", &["dup"]))
            .unwrap();

        let index = mgr.load_merged_index("test").unwrap().unwrap();
        assert!(index.get_document("dup").is_ok());
        assert_eq!(index.list_document_ids().len(), 1);
    }

    #[test]
    fn metadata_persisted_across_restart() {
        let dir = tempfile::tempdir().unwrap();

        {
            let mut mgr = SegmentManager::open(dir.path(), &["idx".into()]).unwrap();
            mgr.write_segment("idx", &make_data("idx", &["x"]))
                .unwrap();
            mgr.write_segment("idx", &make_data("idx", &["y"]))
                .unwrap();
        }

        {
            let mgr = SegmentManager::open(dir.path(), &["idx".into()]).unwrap();
            assert_eq!(mgr.segment_ids("idx"), vec![1, 2]);
            let meta = mgr.registry("idx").unwrap().get(1).unwrap();
            assert_eq!(meta.document_count, 1);
            assert!(meta.size_bytes > 0);
            assert!(meta.created_at > 0);
        }
    }

    #[test]
    fn replace_with_compact_segment() {
        let dir = tempfile::tempdir().unwrap();
        let mut mgr = SegmentManager::open(dir.path(), &["test".into()]).unwrap();

        mgr.write_segment("test", &make_data("test", &["a"]))
            .unwrap();
        mgr.write_segment("test", &make_data("test", &["b"]))
            .unwrap();

        let compact = mgr
            .replace_with_compact_segment(
                "test",
                &make_data("test", &["a", "b"]),
                &[1, 2],
                3,
            )
            .unwrap();

        assert_eq!(compact.id, 3);
        assert_eq!(mgr.segment_ids("test"), vec![3]);
        assert!(!SegmentMetadata::segment_path(&mgr.index_dir("test"), 1).exists());
        assert!(!SegmentMetadata::segment_path(&mgr.index_dir("test"), 2).exists());
    }

    #[test]
    fn sync_to_manifest_updates_ids() {
        let dir = tempfile::tempdir().unwrap();
        let mut mgr = SegmentManager::open(dir.path(), &["prod".into()]).unwrap();
        mgr.write_segment("prod", &make_data("prod", &["a"]))
            .unwrap();
        mgr.write_segment("prod", &make_data("prod", &["b"]))
            .unwrap();

        let mut manifest = Manifest::new();
        mgr.sync_to_manifest(&mut manifest);
        assert_eq!(
            manifest.get_index("prod").unwrap().segments,
            vec![1, 2]
        );
    }

    #[test]
    fn delete_segment_removes_file_and_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let mut mgr = SegmentManager::open(dir.path(), &["test".into()]).unwrap();
        mgr.write_segment("test", &make_data("test", &["a"]))
            .unwrap();

        mgr.delete_segment("test", 1).unwrap();
        assert!(mgr.segment_ids("test").is_empty());
        assert!(!SegmentMetadata::segment_path(&mgr.index_dir("test"), 1).exists());
    }

    #[test]
    fn load_returns_none_for_empty_index() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SegmentManager::open(dir.path(), &["empty".into()]).unwrap();
        assert!(mgr.load_merged_index("empty").unwrap().is_none());
    }

    #[test]
    fn remove_index_clears_registry() {
        let dir = tempfile::tempdir().unwrap();
        let mut mgr = SegmentManager::open(dir.path(), &["gone".into()]).unwrap();
        mgr.write_segment("gone", &make_data("gone", &["a"]))
            .unwrap();
        mgr.remove_index("gone");
        assert!(mgr.registry("gone").is_none());
    }
}
