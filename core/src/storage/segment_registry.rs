use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::segment::Segment;
use super::segment_metadata::{SegmentMetadata, SegmentState};

/// On-disk filename for persisted segment metadata within an index directory.
pub const SEGMENTS_META_FILENAME: &str = "segments.json";

/// In-memory registry of all segments belonging to a single index.
///
/// The registry is the authoritative list of segment metadata for an index.
/// It is persisted to `{index_dir}/segments.json` and reconciled with
/// discovered `.seg` files on load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SegmentRegistry {
    /// Name of the index this registry belongs to.
    pub index_name: String,
    /// Segments keyed by ID (BTreeMap keeps deterministic iteration order).
    pub segments: BTreeMap<u64, SegmentMetadata>,
}

impl SegmentRegistry {
    /// Create an empty registry for a new index.
    pub fn new(index_name: impl Into<String>) -> Self {
        Self {
            index_name: index_name.into(),
            segments: BTreeMap::new(),
        }
    }

    /// Load a registry from disk, reconciling with discovered segment files.
    ///
    /// If `segments.json` does not exist, builds the registry purely from
    /// on-disk `.seg` files.
    pub fn load(index_dir: &Path, index_name: &str) -> io::Result<Self> {
        let meta_path = index_dir.join(SEGMENTS_META_FILENAME);
        let mut registry = if meta_path.exists() {
            let json = fs::read_to_string(&meta_path)?;
            serde_json::from_str(&json).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("segments.json parse error: {e}"),
                )
            })?
        } else {
            Self::new(index_name)
        };

        // Ensure index name is consistent
        registry.index_name = index_name.to_string();
        registry.reconcile_with_disk(index_dir)?;
        Ok(registry)
    }

    /// Persist the registry atomically to `{index_dir}/segments.json`.
    pub fn save(&self, index_dir: &Path) -> io::Result<()> {
        fs::create_dir_all(index_dir)?;
        let path = index_dir.join(SEGMENTS_META_FILENAME);
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("segments.json serialization error: {e}"),
            )
        })?;
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, json)?;
        fs::rename(tmp, path)?;
        Ok(())
    }

    /// Synchronize registry entries with segment files on disk.
    ///
    /// - Adds metadata for `.seg` files not yet tracked.
    /// - Removes entries whose files no longer exist (unless marked deleted).
    pub fn reconcile_with_disk(&mut self, index_dir: &Path) -> io::Result<()> {
        let discovered = Segment::discover(index_dir)?;

        for seg in &discovered {
            if !self.segments.contains_key(&seg.id) {
                let meta = SegmentMetadata::from_disk_segment(seg)?;
                self.segments.insert(seg.id, meta);
            } else if let Some(entry) = self.segments.get_mut(&seg.id) {
                // Refresh size and document count from disk
                let fresh = SegmentMetadata::from_disk_segment(seg)?;
                entry.document_count = fresh.document_count;
                entry.size_bytes = fresh.size_bytes;
            }
        }

        // Drop entries whose files are gone (keep deleted markers out)
        self.segments.retain(|id, meta| {
            if meta.state == SegmentState::Deleted {
                return false;
            }
            SegmentMetadata::segment_path(index_dir, *id).exists()
        });

        Ok(())
    }

    /// Register or replace metadata for a segment.
    pub fn register(&mut self, meta: SegmentMetadata) {
        self.segments.insert(meta.id, meta);
    }

    /// Remove a segment from the registry.
    pub fn remove(&mut self, id: u64) -> Option<SegmentMetadata> {
        self.segments.remove(&id)
    }

    /// Look up metadata for a segment ID.
    pub fn get(&self, id: u64) -> Option<&SegmentMetadata> {
        self.segments.get(&id)
    }

    /// All segments sorted by ID (ascending).
    pub fn list(&self) -> Vec<&SegmentMetadata> {
        self.segments.values().collect()
    }

    /// Active segments sorted by ID (ascending).
    pub fn active_segments(&self) -> Vec<&SegmentMetadata> {
        self.segments
            .values()
            .filter(|m| m.is_active())
            .collect()
    }

    /// Sorted list of active segment IDs.
    pub fn active_segment_ids(&self) -> Vec<u64> {
        self.active_segments().into_iter().map(|m| m.id).collect()
    }

    /// Total documents across all active segments.
    pub fn total_documents(&self) -> u64 {
        self.active_segments()
            .iter()
            .map(|m| m.document_count)
            .sum()
    }

    /// Total on-disk size of all active segments.
    pub fn total_size_bytes(&self) -> u64 {
        self.active_segments()
            .iter()
            .map(|m| m.size_bytes)
            .sum()
    }

    /// Next segment ID (max existing + 1, or 1 if empty).
    pub fn next_segment_id(&self) -> u64 {
        self.segments.keys().last().copied().unwrap_or(0) + 1
    }

    /// Number of active segments.
    pub fn active_count(&self) -> usize {
        self.active_segments().len()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::index::inverted::InvertedIndex;
    use crate::ranking::statistics::CollectionStats;
    use crate::schema::Mapping;
    use crate::storage::segment::{IndexSegmentData, Segment};

    use super::*;

    fn sample_data(name: &str) -> IndexSegmentData {
        let mut documents = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("hello"));
        let doc = Document::new("doc1", fields).unwrap();
        documents.insert("doc1".to_string(), doc);

        IndexSegmentData {
            name: name.into(),
            mapping: Mapping::new(vec![]),
            documents,
            inverted_index: InvertedIndex::new(),
            stats: CollectionStats::new(),
        }
    }

    #[test]
    fn register_and_list_segments() {
        let mut reg = SegmentRegistry::new("products");
        reg.register(SegmentMetadata::new(1, 10, 100));
        reg.register(SegmentMetadata::new(2, 20, 200));

        assert_eq!(reg.list().len(), 2);
        assert_eq!(reg.active_segment_ids(), vec![1, 2]);
        assert_eq!(reg.total_documents(), 30);
        assert_eq!(reg.total_size_bytes(), 300);
    }

    #[test]
    fn next_segment_id_increments() {
        let mut reg = SegmentRegistry::new("test");
        assert_eq!(reg.next_segment_id(), 1);
        reg.register(SegmentMetadata::new(1, 0, 0));
        assert_eq!(reg.next_segment_id(), 2);
        reg.register(SegmentMetadata::new(5, 0, 0));
        assert_eq!(reg.next_segment_id(), 6);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let index_dir = dir.path().join("products");
        fs::create_dir_all(&index_dir).unwrap();

        // Metadata requires corresponding segment files on disk
        Segment::write_segment(&index_dir, 1, &sample_data("products")).unwrap();
        Segment::write_segment(&index_dir, 2, &sample_data("products")).unwrap();

        let mut reg = SegmentRegistry::new("products");
        reg.register(SegmentMetadata::new(1, 5, 512));
        reg.register(SegmentMetadata::new(2, 10, 1024));
        reg.save(&index_dir).unwrap();

        let loaded = SegmentRegistry::load(&index_dir, "products").unwrap();
        assert_eq!(loaded.index_name, "products");
        assert_eq!(loaded.active_segment_ids(), vec![1, 2]);
        // Reconcile refreshes counts from on-disk segment files (1 doc each)
        assert_eq!(loaded.total_documents(), 2);
    }

    #[test]
    fn load_from_disk_discovers_untracked_segments() {
        let dir = tempfile::tempdir().unwrap();
        let index_dir = dir.path().join("test");
        fs::create_dir_all(&index_dir).unwrap();

        Segment::write_segment(&index_dir, 1, &sample_data("test")).unwrap();
        Segment::write_segment(&index_dir, 2, &sample_data("test")).unwrap();

        let reg = SegmentRegistry::load(&index_dir, "test").unwrap();
        assert_eq!(reg.active_segment_ids(), vec![1, 2]);
        assert_eq!(reg.active_count(), 2);
        assert!(reg.get(1).unwrap().document_count >= 1);
    }

    #[test]
    fn reconcile_removes_stale_entries() {
        let dir = tempfile::tempdir().unwrap();
        let index_dir = dir.path().join("test");
        fs::create_dir_all(&index_dir).unwrap();

        Segment::write_segment(&index_dir, 1, &sample_data("test")).unwrap();

        let mut reg = SegmentRegistry::new("test");
        reg.register(SegmentMetadata::new(1, 1, 100));
        reg.register(SegmentMetadata::new(99, 1, 100)); // file does not exist
        reg.reconcile_with_disk(&index_dir).unwrap();

        assert_eq!(reg.active_segment_ids(), vec![1]);
        assert!(reg.get(99).is_none());
    }

    #[test]
    fn inactive_segments_excluded_from_totals() {
        let mut reg = SegmentRegistry::new("test");
        let mut merging = SegmentMetadata::new(1, 100, 1000);
        merging.state = SegmentState::Merging;
        reg.register(merging);
        reg.register(SegmentMetadata::new(2, 50, 500));

        assert_eq!(reg.active_count(), 1);
        assert_eq!(reg.total_documents(), 50);
        assert_eq!(reg.active_segment_ids(), vec![2]);
    }

    #[test]
    fn metadata_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let index_dir = dir.path().join("idx");
        fs::create_dir_all(&index_dir).unwrap();

        Segment::write_segment(&index_dir, 3, &sample_data("idx")).unwrap();

        {
            let mut reg = SegmentRegistry::new("idx");
            reg.register(SegmentMetadata::new(3, 7, 777));
            reg.save(&index_dir).unwrap();
        }

        {
            let loaded = SegmentRegistry::load(&index_dir, "idx").unwrap();
            let meta = loaded.get(3).unwrap();
            // Reconcile refreshes from disk segment (1 document in sample_data)
            assert_eq!(meta.document_count, 1);
            assert!(meta.size_bytes > 0);
            assert!(meta.created_at > 0);
        }
    }
}
