use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::document::Document;
use crate::index::inverted::InvertedIndex;
use crate::index::Index;
use crate::ranking::statistics::CollectionStats;

use super::segment::IndexSegmentData;

/// Wrapper stored on disk that bundles snapshot data with an integrity checksum.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotFile {
    /// Simple 64-bit checksum (sum of bytes) of `data`'s JSON representation.
    checksum: u64,
    /// The actual index data.
    data: IndexSegmentData,
}

/// A point-in-time snapshot of a single index's full state.
///
/// Snapshots are written atomically and carry a checksum for integrity
/// verification on restore.
#[derive(Debug)]
pub struct Snapshot {
    /// Path to the snapshot file.
    pub path: PathBuf,
    /// The index data contained in this snapshot.
    pub data: IndexSegmentData,
}

impl Snapshot {
    /// Filename used for snapshot files.
    const FILENAME: &'static str = "snapshot.json";

    /// Create a snapshot of an index's current state.
    ///
    /// Writes atomically (.tmp + rename) and includes a checksum for
    /// integrity verification.
    pub fn create_snapshot(dir: impl AsRef<Path>, index: &Index) -> io::Result<Self> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        let path = dir.join(Self::FILENAME);

        let data = IndexSegmentData {
            name: index.name().to_string(),
            mapping: index.mapping().clone(),
            documents: extract_documents(index),
            inverted_index: extract_inverted_index(index),
            stats: extract_stats(index),
        };

        let data_json = serde_json::to_string(&data).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("snapshot serialization error: {e}"))
        })?;

        let checksum = compute_checksum(data_json.as_bytes());
        let snapshot_file = SnapshotFile { checksum, data };

        let json = serde_json::to_string(&snapshot_file).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("snapshot file serialization error: {e}"))
        })?;

        let tmp_path = dir.join(format!("{}.tmp", Self::FILENAME));
        fs::write(&tmp_path, &json)?;
        fs::rename(&tmp_path, &path)?;

        Ok(Self { path, data: snapshot_file.data })
    }

    /// Load a snapshot from disk, verifying its integrity checksum.
    ///
    /// Returns `None` if the snapshot file does not exist.
    /// Returns an error if the checksum does not match (data corruption).
    pub fn load(dir: impl AsRef<Path>) -> io::Result<Option<Self>> {
        let path = dir.as_ref().join(Self::FILENAME);
        if !path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(&path)?;
        let snapshot_file: SnapshotFile = serde_json::from_str(&json).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("snapshot parse error: {e}"))
        })?;

        // Verify integrity
        let data_json = serde_json::to_string(&snapshot_file.data).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("snapshot re-serialization error: {e}"))
        })?;
        let actual_checksum = compute_checksum(data_json.as_bytes());
        if actual_checksum != snapshot_file.checksum {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "snapshot integrity check failed: expected checksum {}, got {}",
                    snapshot_file.checksum, actual_checksum
                ),
            ));
        }

        Ok(Some(Self { path, data: snapshot_file.data }))
    }

    /// Reconstruct an Index from the snapshot data.
    pub fn restore_index(&self) -> Index {
        let mut index = Index::new(&self.data.name, self.data.mapping.clone());
        for (doc_id, doc) in &self.data.documents {
            if index.get_document(doc_id).is_err() {
                let _ = index.add_document(doc.clone());
            }
        }
        index
    }
}

/// Compute a deterministic 64-bit checksum from a byte slice.
///
/// Uses a simple sum-of-bytes approach that is stable across Rust versions
/// and process invocations.
fn compute_checksum(bytes: &[u8]) -> u64 {
    let mut sum: u64 = 0;
    for &b in bytes {
        sum = sum.wrapping_add(b as u64);
    }
    sum
}

fn extract_documents(index: &Index) -> std::collections::HashMap<String, Document> {
    let mut docs = std::collections::HashMap::new();
    for id in index.list_document_ids() {
        if let Ok(doc) = index.get_document(&id) {
            docs.insert(id, doc.clone());
        }
    }
    docs
}

fn extract_inverted_index(index: &Index) -> InvertedIndex {
    index.inverted_index_clone()
}

fn extract_stats(index: &Index) -> CollectionStats {
    index.stats_ref().clone()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::index::Index;
    use crate::schema::Mapping;

    use super::*;

    fn setup_index() -> Index {
        let mut index = Index::new("test", Mapping::new(vec![]));
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("hello world"));
        let doc = Document::new("doc1", fields).unwrap();
        index.add_document(doc).unwrap();
        index
    }

    #[test]
    fn create_and_load_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let index = setup_index();

        let snap = Snapshot::create_snapshot(dir.path(), &index).unwrap();
        assert!(snap.path.exists());

        let loaded = Snapshot::load(dir.path()).unwrap().expect("snapshot should exist");
        assert_eq!(loaded.data.name, "test");
        assert_eq!(loaded.data.documents.len(), 1);
    }

    #[test]
    fn restore_index_from_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let index = setup_index();

        Snapshot::create_snapshot(dir.path(), &index).unwrap();
        let loaded = Snapshot::load(dir.path()).unwrap().unwrap();
        let restored = loaded.restore_index();

        assert_eq!(restored.name(), "test");
        assert!(restored.get_document("doc1").is_ok());
    }

    #[test]
    fn load_nonexistent_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let result = Snapshot::load(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn snapshot_created_verified_restored() {
        let dir = tempfile::tempdir().unwrap();
        let mut index = setup_index();

        // Add a second document
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!("second"));
        let doc = Document::new("doc2", fields).unwrap();
        index.add_document(doc).unwrap();

        Snapshot::create_snapshot(dir.path(), &index).unwrap();
        let loaded = Snapshot::load(dir.path()).unwrap().unwrap();
        let restored = loaded.restore_index();

        assert_eq!(restored.name(), "test");
        assert!(restored.get_document("doc1").is_ok());
        assert!(restored.get_document("doc2").is_ok());
    }

    #[test]
    fn tampered_snapshot_detected() {
        let dir = tempfile::tempdir().unwrap();
        let index = setup_index();

        Snapshot::create_snapshot(dir.path(), &index).unwrap();

        // Corrupt the snapshot file — change a field value (JSON stays valid)
        let path = dir.path().join("snapshot.json");
        let content = fs::read_to_string(&path).unwrap();
        let corrupted = content.replace("\"hello world\"", "\"corrupted!\"");
        assert_ne!(content, corrupted, "corruption must change content");
        fs::write(&path, &corrupted).unwrap();

        let result = Snapshot::load(dir.path());
        assert!(result.is_err(), "expected error for corrupted snapshot");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("integrity check failed"),
            "expected integrity error, got: {err}"
        );
    }

    #[test]
    fn snapshot_survives_restart() {
        let dir = tempfile::tempdir().unwrap();

        // First session
        {
            let index = setup_index();
            Snapshot::create_snapshot(dir.path(), &index).unwrap();
        }

        // Second session — reopen
        {
            let loaded = Snapshot::load(dir.path()).unwrap().expect("snapshot should survive");
            let restored = loaded.restore_index();
            assert_eq!(restored.name(), "test");
            assert!(restored.get_document("doc1").is_ok());
        }
    }

    #[test]
    fn create_empty_index_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let index = Index::new("empty", Mapping::new(vec![]));

        Snapshot::create_snapshot(dir.path(), &index).unwrap();

        let loaded = Snapshot::load(dir.path()).unwrap().unwrap();
        let restored = loaded.restore_index();
        assert_eq!(restored.name(), "empty");
        assert!(restored.list_document_ids().is_empty());
    }
}
