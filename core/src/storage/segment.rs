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
}
