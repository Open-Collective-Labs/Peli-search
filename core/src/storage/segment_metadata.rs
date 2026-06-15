use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::segment::{IndexSegmentData, Segment};

/// Lifecycle state of a segment on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SegmentState {
    /// Searchable and included in recovery.
    #[default]
    Active,
    /// Being merged into a new segment; excluded from search/recovery.
    Merging,
    /// Marked for deletion; excluded from search/recovery.
    Deleted,
}

/// Metadata describing a single on-disk segment.
///
/// Persisted in `{index_dir}/segments.json` and mirrored in the root
/// manifest's segment ID list for backward compatibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SegmentMetadata {
    /// Monotonically increasing segment identifier within an index.
    pub id: u64,
    /// Number of documents stored in this segment.
    pub document_count: u64,
    /// Unix timestamp (seconds) when the segment was created.
    pub created_at: u64,
    /// On-disk file size in bytes.
    pub size_bytes: u64,
    /// Current lifecycle state.
    #[serde(default)]
    pub state: SegmentState,
}

impl SegmentMetadata {
    /// Create metadata for a newly written segment.
    pub fn new(id: u64, document_count: u64, size_bytes: u64) -> Self {
        Self {
            id,
            document_count,
            created_at: now_secs(),
            size_bytes,
            state: SegmentState::Active,
        }
    }

    /// Build metadata from segment payload without touching disk.
    pub fn from_segment_data(id: u64, size_bytes: u64, data: &IndexSegmentData) -> Self {
        Self {
            id,
            document_count: data.documents.len() as u64,
            created_at: now_secs(),
            size_bytes,
            state: SegmentState::Active,
        }
    }

    /// Read a segment file from disk and derive its metadata.
    pub fn from_disk_segment(segment: &Segment) -> io::Result<Self> {
        let size_bytes = fs::metadata(&segment.path)?.len();
        let data = Segment::read_segment(&segment.path)?;
        Ok(Self::from_segment_data(segment.id, size_bytes, &data))
    }

    /// Path to a segment file within an index directory.
    pub fn segment_path(index_dir: &Path, id: u64) -> PathBuf {
        index_dir.join(format!("{id:06}.seg"))
    }

    /// Whether this segment should participate in search and recovery.
    pub fn is_active(&self) -> bool {
        self.state == SegmentState::Active
    }
}

/// Current Unix timestamp in seconds.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::index::inverted::InvertedIndex;
    use crate::ranking::statistics::CollectionStats;
    use crate::schema::Mapping;

    use super::*;

    fn sample_data() -> IndexSegmentData {
        let mut documents = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("hello"));
        let doc = Document::new("doc1", fields).unwrap();
        documents.insert("doc1".to_string(), doc);

        IndexSegmentData {
            name: "test".into(),
            mapping: Mapping::new(vec![]),
            documents,
            inverted_index: InvertedIndex::new(),
            stats: CollectionStats::new(),
        }
    }

    #[test]
    fn new_metadata_fields() {
        let meta = SegmentMetadata::new(1, 42, 1024);
        assert_eq!(meta.id, 1);
        assert_eq!(meta.document_count, 42);
        assert_eq!(meta.size_bytes, 1024);
        assert_eq!(meta.state, SegmentState::Active);
        assert!(meta.created_at > 0);
    }

    #[test]
    fn from_segment_data_counts_documents() {
        let data = sample_data();
        let meta = SegmentMetadata::from_segment_data(3, 512, &data);
        assert_eq!(meta.id, 3);
        assert_eq!(meta.document_count, 1);
        assert_eq!(meta.size_bytes, 512);
    }

    #[test]
    fn from_disk_segment_reads_file() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();
        let seg = Segment::write_segment(dir.path(), 7, &data).unwrap();

        let meta = SegmentMetadata::from_disk_segment(&seg).unwrap();
        assert_eq!(meta.id, 7);
        assert_eq!(meta.document_count, 1);
        assert!(meta.size_bytes > 0);
    }

    #[test]
    fn segment_path_format() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            SegmentMetadata::segment_path(dir.path(), 42),
            dir.path().join("000042.seg")
        );
    }

    #[test]
    fn metadata_serializes() {
        let meta = SegmentMetadata::new(1, 10, 2048);
        let json = serde_json::to_string(&meta).unwrap();
        let loaded: SegmentMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, meta);
    }

    #[test]
    fn is_active_respects_state() {
        let mut meta = SegmentMetadata::new(1, 0, 0);
        assert!(meta.is_active());
        meta.state = SegmentState::Merging;
        assert!(!meta.is_active());
        meta.state = SegmentState::Deleted;
        assert!(!meta.is_active());
    }
}
