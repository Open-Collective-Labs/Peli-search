use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::document::Document;
use crate::index::inverted::InvertedIndex;
use crate::ranking::statistics::CollectionStats;
use crate::schema::Mapping;

/// A serializable snapshot of an index's full state at a point in time.
///
/// Stores documents, mappings, statistics, and postings lists so an index
/// can be fully reconstructed from disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSegmentData {
    /// Index name.
    pub name: String,
    /// Schema mapping.
    pub mapping: Mapping,
    /// All stored documents keyed by document ID.
    pub documents: std::collections::HashMap<String, Document>,
    /// Inverted index state (term → postings lists).
    pub inverted_index: InvertedIndex,
    /// Collection statistics for ranking.
    pub stats: CollectionStats,
}

/// A persistent segment file containing the full state of an index.
///
/// Each segment is a single JSON file written atomically. The segment
/// number increases monotonically within an index's directory.
///
/// # Data stored per segment
///
/// - Documents (all stored documents keyed by document ID)
/// - Mappings (schema field definitions)
/// - Statistics (collection-level stats for BM25 ranking)
/// - Postings Lists (inverted index: term → document IDs)
#[derive(Debug)]
pub struct Segment {
    /// Monotonically increasing segment number.
    pub id: u64,
    /// Path to the segment file.
    pub path: PathBuf,
}

impl Segment {
    /// Write an index segment to disk at the given path with the given ID.
    ///
    /// The segment is written atomically: data is first written to a `.tmp`
    /// file, then renamed to the final path. This prevents partial writes
    /// from producing a corrupt segment file.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the directory cannot be created or the file
    /// cannot be written.
    pub fn write_segment(
        dir: impl AsRef<Path>,
        id: u64,
        data: &IndexSegmentData,
    ) -> io::Result<Self> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        let filename = format!("{id:06}.seg");
        let path = dir.join(&filename);

        // Immutability enforcement: once a segment file exists, it must never
        // be overwritten. Updates must create a new segment with a new ID.
        if path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "segment {} already exists at {}; segments are immutable and cannot be overwritten",
                    id,
                    path.display()
                ),
            ));
        }

        let json = serde_json::to_string(data).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("serialization error: {e}"))
        })?;

        // Atomic write: write to .tmp, then rename
        let tmp_path = dir.join(format!("{filename}.tmp"));
        fs::write(&tmp_path, &json)?;
        fs::rename(&tmp_path, &path)?;

        Ok(Self { id, path })
    }

    /// Read and deserialize index data from a segment file.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the file does not exist, cannot be read, or
    /// contains invalid data.
    pub fn read_segment(path: impl AsRef<Path>) -> io::Result<IndexSegmentData> {
        let json = fs::read_to_string(path.as_ref())?;
        let data: IndexSegmentData = serde_json::from_str(&json).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("deserialization error: {e}"))
        })?;
        Ok(data)
    }

    /// Delete a segment file from disk.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the file could not be deleted. Returns `Ok(())`
    /// if the file does not exist.
    pub fn delete_segment(path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Check whether this segment is immutable (i.e., its file exists on disk).
    ///
    /// A segment that exists on disk is immutable: its contents cannot be
    /// changed. All updates must create a new segment with a new ID.
    pub fn is_immutable(&self) -> bool {
        self.path.exists()
    }

    /// Verify that this segment is immutable. Returns an error if the
    /// segment file does not exist on disk.
    ///
    /// Use this before attempting reads to ensure the segment is in a
    /// valid, finalized state.
    pub fn ensure_immutable(&self) -> io::Result<()> {
        if !self.path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "segment {} does not exist at {}; cannot read from a non-existent segment",
                    self.id,
                    self.path.display()
                ),
            ));
        }
        Ok(())
    }

    /// Discover all segment files in a directory, returning them sorted by
    /// segment ID (oldest first).
    pub fn discover(dir: impl AsRef<Path>) -> io::Result<Vec<Self>> {
        let dir = dir.as_ref();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut segments = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "seg") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(id) = stem.parse::<u64>() {
                        segments.push(Self { id, path });
                    }
                }
            }
        }

        segments.sort_by_key(|s| s.id);
        Ok(segments)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::index::inverted::InvertedIndex;
    use crate::ranking::statistics::CollectionStats;
    use crate::schema::Mapping;

    use super::*;

    #[test]
    fn write_segment_rejects_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();

        // First write succeeds
        Segment::write_segment(dir.path(), 1, &data).unwrap();

        // Second write with same ID must fail (immutability)
        let err = Segment::write_segment(dir.path(), 1, &data).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        let msg = format!("{}", err);
        assert!(msg.contains("immutable"), "error should mention immutability: {msg}");
    }

    #[test]
    fn write_segment_different_ids_always_work() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();

        // Writing different IDs should always succeed
        Segment::write_segment(dir.path(), 1, &data).unwrap();
        Segment::write_segment(dir.path(), 2, &data).unwrap();
        Segment::write_segment(dir.path(), 3, &data).unwrap();

        let segments = Segment::discover(dir.path()).unwrap();
        assert_eq!(segments.len(), 3);
    }

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
    fn write_and_read_segment() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();

        let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
        assert_eq!(seg.id, 1);
        assert!(seg.path.exists());

        let loaded = Segment::read_segment(&seg.path).unwrap();
        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.documents.len(), 1);
        assert!(loaded.documents.contains_key("doc1"));
    }

    #[test]
    fn read_segment_returns_all_fields() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();

        let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
        let loaded = Segment::read_segment(&seg.path).unwrap();

        // Documents
        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.documents.len(), 1);
        assert_eq!(loaded.documents["doc1"].id, "doc1");

        // Mappings
        assert!(loaded.mapping.fields().is_empty());

        // Statistics
        assert_eq!(loaded.stats.total_documents(), 0);

        // Postings lists (inverted index)
        // No documents were indexed yet, so postings should be empty
    }

    #[test]
    fn write_segment_stores_documents() {
        let dir = tempfile::tempdir().unwrap();
        let mut documents = HashMap::new();

        let mut fields1 = HashMap::new();
        fields1.insert("title".to_string(), serde_json::json!("first"));
        let doc1 = Document::new("doc1", fields1).unwrap();
        documents.insert("doc1".to_string(), doc1);

        let mut fields2 = HashMap::new();
        fields2.insert("title".to_string(), serde_json::json!("second"));
        fields2.insert("price".to_string(), serde_json::json!(100));
        let doc2 = Document::new("doc2", fields2).unwrap();
        documents.insert("doc2".to_string(), doc2);

        let data = IndexSegmentData {
            name: "products".into(),
            mapping: Mapping::new(vec![]),
            documents,
            inverted_index: InvertedIndex::new(),
            stats: CollectionStats::new(),
        };

        let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
        let loaded = Segment::read_segment(&seg.path).unwrap();

        assert_eq!(loaded.documents.len(), 2);
        assert_eq!(loaded.documents["doc1"].get_field("title").unwrap(), "first");
        assert_eq!(
            loaded.documents["doc2"].get_field("price").unwrap(),
            &serde_json::json!(100)
        );
    }

    #[test]
    fn write_segment_stores_mappings() {
        let dir = tempfile::tempdir().unwrap();
        use crate::schema::{Field, FieldType};

        let mapping = Mapping::new(vec![
            Field::new("title", FieldType::Text, true),
            Field::new("price", FieldType::Float, false),
        ]);

        let data = IndexSegmentData {
            name: "test".into(),
            mapping,
            documents: HashMap::new(),
            inverted_index: InvertedIndex::new(),
            stats: CollectionStats::new(),
        };

        let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
        let loaded = Segment::read_segment(&seg.path).unwrap();

        assert!(loaded.mapping.field_exists("title"));
        assert!(loaded.mapping.field_exists("price"));
        assert_eq!(loaded.mapping.fields().len(), 2);
    }

    #[test]
    fn write_segment_stores_statistics() {
        let dir = tempfile::tempdir().unwrap();
        let mut stats = CollectionStats::new();
        stats.update_document("doc1", "hello world");
        stats.update_document("doc2", "hello");

        let data = IndexSegmentData {
            name: "test".into(),
            mapping: Mapping::new(vec![]),
            documents: HashMap::new(),
            inverted_index: InvertedIndex::new(),
            stats,
        };

        let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
        let loaded = Segment::read_segment(&seg.path).unwrap();

        assert_eq!(loaded.stats.total_documents(), 2);
        assert_eq!(loaded.stats.average_document_length(), 1.5);
    }

    #[test]
    fn write_segment_stores_postings() {
        let dir = tempfile::tempdir().unwrap();
        let mut index = crate::index::Index::new("test", Mapping::new(vec![]));

        let mut fields1 = HashMap::new();
        fields1.insert("title".to_string(), serde_json::json!("hello world"));
        let doc1 = Document::new("doc1", fields1).unwrap();
        index.add_document(doc1).unwrap();

        let mut fields2 = HashMap::new();
        fields2.insert("title".to_string(), serde_json::json!("hello there"));
        let doc2 = Document::new("doc2", fields2).unwrap();
        index.add_document(doc2).unwrap();

        let data = IndexSegmentData {
            name: "test".into(),
            mapping: Mapping::new(vec![]),
            documents: HashMap::new(),
            inverted_index: index.inverted_index_clone(),
            stats: index.stats_ref().clone(),
        };

        let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
        let loaded = Segment::read_segment(&seg.path).unwrap();

        // Verify postings: "hello" should reference both docs
        let hello_postings = loaded.inverted_index.get_postings("hello");
        assert!(hello_postings.is_some(), "postings for 'hello' should exist");
        let postings = hello_postings.unwrap();
        assert_eq!(postings.len(), 2);
        assert!(postings.contains(&"doc1".to_string()));
        assert!(postings.contains(&"doc2".to_string()));

        // "world" should reference only doc1
        let world_postings = loaded.inverted_index.get_postings("world").unwrap();
        assert_eq!(world_postings.len(), 1);
        assert!(world_postings.contains(&"doc1".to_string()));
    }

    #[test]
    fn delete_segment_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();

        let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
        assert!(seg.path.exists());

        Segment::delete_segment(&seg.path).unwrap();
        assert!(!seg.path.exists());
    }

    #[test]
    fn delete_nonexistent_segment_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("999999.seg");
        // Should not error
        Segment::delete_segment(&path).unwrap();
    }

    #[test]
    fn data_preserved_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();

        // First session: write segment
        let path = {
            let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
            seg.path.clone()
        };

        // Simulate restart: drop everything, reopen by path
        let loaded = Segment::read_segment(&path).unwrap();
        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.documents.len(), 1);
        assert_eq!(loaded.documents["doc1"].id, "doc1");
    }

    #[test]
    fn atomic_write_no_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();

        // Write a corrupt .tmp file to simulate a previous crash
        let tmp_path = dir.path().join("000001.seg.tmp");
        fs::write(&tmp_path, "corrupt garbage data").unwrap();

        // Proper write should overwrite the .tmp and produce a valid .seg
        let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
        let loaded = Segment::read_segment(&seg.path).unwrap();
        assert_eq!(loaded.name, "test");

        // The .tmp file should be gone (rename replaces it)
        assert!(!tmp_path.exists());
    }

    #[test]
    fn discover_segments_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();

        Segment::write_segment(dir.path(), 2, &data).unwrap();
        Segment::write_segment(dir.path(), 1, &data).unwrap();
        Segment::write_segment(dir.path(), 3, &data).unwrap();

        let segments = Segment::discover(dir.path()).unwrap();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].id, 1);
        assert_eq!(segments[1].id, 2);
        assert_eq!(segments[2].id, 3);
    }

    #[test]
    fn discover_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let segments = Segment::discover(dir.path()).unwrap();
        assert!(segments.is_empty());
    }

    #[test]
    fn discover_nonexistent_directory() {
        let segments = Segment::discover("/tmp/nonexistent_seg_dir_xyz").unwrap();
        assert!(segments.is_empty());
    }

    #[test]
    fn segment_is_immutable_after_write() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();

        let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
        assert!(seg.is_immutable(), "segment should be immutable after writing");
    }

    #[test]
    fn segment_not_immutable_before_write() {
        let dir = tempfile::tempdir().unwrap();
        let seg = Segment {
            id: 1,
            path: dir.path().join("000001.seg"),
        };
        assert!(!seg.is_immutable(), "segment should not be immutable before writing");
    }

    #[test]
    fn ensure_immutable_succeeds_for_existing_segment() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();

        let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
        assert!(seg.ensure_immutable().is_ok());
    }

    #[test]
    fn ensure_immutable_fails_for_nonexistent_segment() {
        let dir = tempfile::tempdir().unwrap();
        let seg = Segment {
            id: 1,
            path: dir.path().join("000001.seg"),
        };
        let err = seg.ensure_immutable().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(format!("{}", err).contains("does not exist"));
    }

    #[test]
    fn immutable_segment_rejects_overwrite_with_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();

        Segment::write_segment(dir.path(), 1, &data).unwrap();
        let err = Segment::write_segment(dir.path(), 1, &data).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        let msg = format!("{}", err);
        assert!(msg.contains("immutable"), "error must mention immutability: {msg}");
        assert!(msg.contains("1"), "error must mention segment ID: {msg}");
    }

    #[test]
    fn immutable_segments_cannot_be_modified_via_write() {
        let dir = tempfile::tempdir().unwrap();
        let mut data1 = sample_data();
        let mut data2 = sample_data();

        // First write
        data1.documents.insert(
            "doc_orig".to_string(),
            Document::new("doc_orig", HashMap::from([("title".to_string(), serde_json::json!("original"))])).unwrap(),
        );
        let seg = Segment::write_segment(dir.path(), 1, &data1).unwrap();

        // Attempt to overwrite with different data
        data2.documents.insert(
            "doc_new".to_string(),
            Document::new("doc_new", HashMap::from([("title".to_string(), serde_json::json!("modified"))])).unwrap(),
        );
        Segment::write_segment(dir.path(), 1, &data2).unwrap_err();

        // Original data must be preserved
        let loaded = Segment::read_segment(&seg.path).unwrap();
        assert!(loaded.documents.contains_key("doc_orig"), "original data must be preserved");
        assert!(!loaded.documents.contains_key("doc_new"), "new data must not overwrite");
    }

    #[test]
    fn updates_must_create_new_segment() {
        let dir = tempfile::tempdir().unwrap();

        // Segment 1: original data
        let mut data1 = sample_data();
        data1.documents.insert(
            "doc1".to_string(),
            Document::new("doc1", HashMap::from([("title".to_string(), serde_json::json!("v1"))])).unwrap(),
        );
        let seg1 = Segment::write_segment(dir.path(), 1, &data1).unwrap();

        // Segment 2: updated data (new segment, new ID)
        let mut data2 = sample_data();
        data2.documents.insert(
            "doc1".to_string(),
            Document::new("doc1", HashMap::from([("title".to_string(), serde_json::json!("v2"))])).unwrap(),
        );
        data2.documents.insert(
            "doc2".to_string(),
            Document::new("doc2", HashMap::from([("title".to_string(), serde_json::json!("new doc"))])).unwrap(),
        );
        let seg2 = Segment::write_segment(dir.path(), 2, &data2).unwrap();

        // Both segments exist independently
        assert!(seg1.is_immutable());
        assert!(seg2.is_immutable());

        let loaded1 = Segment::read_segment(&seg1.path).unwrap();
        let loaded2 = Segment::read_segment(&seg2.path).unwrap();

        // Segment 1 has original doc1
        assert!(loaded1.documents.contains_key("doc1"));
        assert!(!loaded1.documents.contains_key("doc2"));

        // Segment 2 has updated doc1 + new doc2
        assert!(loaded2.documents.contains_key("doc1"));
        assert!(loaded2.documents.contains_key("doc2"));
    }

    #[test]
    fn immutable_segment_data_preserved_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();

        // Write segment in first session
        let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
        let original_json = fs::read_to_string(&seg.path).unwrap();

        // Simulate restart: read back from disk
        let loaded = Segment::read_segment(&seg.path).unwrap();
        let restored_json = serde_json::to_string(&loaded).unwrap();

        // Data must be byte-for-byte identical
        assert_eq!(original_json, restored_json, "immutable segment data must survive restart");
    }

    #[test]
    fn concurrent_reads_of_immutable_segment() {
        use std::sync::Arc;
        use std::thread;

        let dir = tempfile::tempdir().unwrap();
        let data = sample_data();
        let seg = Segment::write_segment(dir.path(), 1, &data).unwrap();
        let path = Arc::new(seg.path.clone());

        let mut handles = vec![];
        for _ in 0..10 {
            let p = Arc::clone(&path);
            handles.push(thread::spawn(move || {
                let loaded = Segment::read_segment(&*p).unwrap();
                assert_eq!(loaded.name, "test");
                assert_eq!(loaded.documents.len(), 1);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
    }
}
