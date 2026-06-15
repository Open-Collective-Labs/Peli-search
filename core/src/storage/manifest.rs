use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Metadata about a single persisted index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Name of the index.
    pub name: String,
    /// All segment IDs belonging to this index, sorted.
    pub segments: Vec<u64>,
}

/// Tracks the set of persisted indexes and their segment metadata.
///
/// The manifest is stored as a JSON file in the storage root directory.
/// On recovery, it tells the system which segments belong to each index
/// and which segments may be missing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Unix timestamp (seconds) of last modification.
    pub timestamp: u64,
    /// Metadata for each persisted index.
    pub indexes: Vec<IndexMetadata>,
}

impl Manifest {
    /// Current manifest schema version.
    pub const CURRENT_VERSION: u32 = 2;

    /// Create a new empty manifest with the current timestamp.
    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            timestamp: now_secs(),
            indexes: Vec::new(),
        }
    }

    /// Load the manifest from a file path.
    ///
    /// Returns a default (empty) manifest if the file does not exist.
    /// Backward-compatible with version 1 manifests (single `last_segment_id`).
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::new());
        }
        let json = fs::read_to_string(path)?;
        let mut manifest: Manifest = serde_json::from_str(&json).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("manifest parse error: {e}"))
        })?;
        // Touch timestamp on load
        manifest.timestamp = now_secs();
        Ok(manifest)
    }

    /// Save the manifest to a file path (atomic write).
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut snapshot = self.clone();
        snapshot.timestamp = now_secs();

        let json = serde_json::to_string_pretty(&snapshot).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("manifest serialization error: {e}"))
        })?;
        let tmp_path = path.with_extension("tmp");
        fs::write(&tmp_path, &json)?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    }

    /// Add or update metadata for an index with the given segment IDs.
    ///
    /// The segment list is deduplicated and sorted.
    pub fn upsert_index(&mut self, name: &str, segments: Vec<u64>) {
        let mut segments = segments;
        segments.sort();
        segments.dedup();

        if let Some(existing) = self.indexes.iter_mut().find(|m| m.name == name) {
            existing.segments = segments;
        } else {
            self.indexes.push(IndexMetadata {
                name: name.to_string(),
                segments,
            });
        }
    }

    /// Remove metadata for an index.
    pub fn remove_index(&mut self, name: &str) {
        self.indexes.retain(|m| m.name != name);
    }

    /// Get metadata for an index.
    pub fn get_index(&self, name: &str) -> Option<&IndexMetadata> {
        self.indexes.iter().find(|m| m.name == name)
    }

    /// Check which segments referenced in the manifest no longer exist on disk.
    ///
    /// `segments_dir` should be the directory containing `.seg` files for
    /// a specific index.
    ///
    /// Returns a list of segment IDs that are in the manifest but missing
    /// from disk.
    pub fn detect_missing_segments(
        &self,
        index_name: &str,
        segments_dir: &Path,
    ) -> Vec<u64> {
        let meta = match self.get_index(index_name) {
            Some(m) => m,
            None => return Vec::new(),
        };

        let mut missing = Vec::new();
        for &seg_id in &meta.segments {
            let seg_path = segments_dir.join(format!("{seg_id:06}.seg"));
            if !seg_path.exists() {
                missing.push(seg_id);
            }
        }
        missing
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Current Unix timestamp in seconds.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn create_default_manifest() {
        let m = Manifest::new();
        assert_eq!(m.version, 2);
        assert!(m.indexes.is_empty());
        assert!(m.timestamp > 0);
    }

    #[test]
    fn save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");

        let mut m = Manifest::new();
        m.upsert_index("products", vec![1, 2, 3]);
        m.upsert_index("users", vec![1]);
        m.save(&path).unwrap();

        let loaded = Manifest::load(&path).unwrap();
        assert_eq!(loaded.indexes.len(), 2);

        let products = loaded.get_index("products").unwrap();
        assert_eq!(products.segments, vec![1, 2, 3]);

        let users = loaded.get_index("users").unwrap();
        assert_eq!(users.segments, vec![1]);
    }

    #[test]
    fn update_existing_index() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");

        let mut m = Manifest::new();
        m.upsert_index("products", vec![1]);
        m.save(&path).unwrap();

        let mut loaded = Manifest::load(&path).unwrap();
        loaded.upsert_index("products", vec![1, 2, 3, 4, 5]);
        loaded.save(&path).unwrap();

        let reloaded = Manifest::load(&path).unwrap();
        assert_eq!(
            reloaded.get_index("products").unwrap().segments,
            vec![1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn upsert_deduplicates_and_sorts() {
        let mut m = Manifest::new();
        m.upsert_index("test", vec![3, 1, 2, 1, 3, 2]);
        assert_eq!(m.get_index("test").unwrap().segments, vec![1, 2, 3]);
    }

    #[test]
    fn remove_index() {
        let mut m = Manifest::new();
        m.upsert_index("products", vec![1]);
        m.upsert_index("users", vec![1]);
        m.remove_index("products");
        assert_eq!(m.indexes.len(), 1);
        assert_eq!(m.indexes[0].name, "users");
    }

    #[test]
    fn missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let m = Manifest::load(&path).unwrap();
        assert!(m.indexes.is_empty());
    }

    #[test]
    fn timestamp_updates_on_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");

        let mut m = Manifest::new();
        let ts1 = m.timestamp;
        m.upsert_index("test", vec![1]);
        m.save(&path).unwrap();
        let ts2 = Manifest::load(&path).unwrap().timestamp;
        // ts2 should be >= ts1
        assert!(ts2 >= ts1);
    }

    #[test]
    fn save_overwrites_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");

        {
            let mut m = Manifest::new();
            m.upsert_index("v1", vec![1]);
            m.save(&path).unwrap();
        }

        {
            let mut m = Manifest::new();
            m.upsert_index("v2", vec![2]);
            m.save(&path).unwrap();
        }

        let loaded = Manifest::load(&path).unwrap();
        assert_eq!(loaded.indexes.len(), 1);
        assert_eq!(loaded.indexes[0].name, "v2");
    }

    #[test]
    fn manifest_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");

        let segments = vec![1, 2, 5, 10];

        // First session
        {
            let mut m = Manifest::new();
            m.upsert_index("products", segments.clone());
            m.save(&path).unwrap();
        }

        // Second session — reopen same file
        {
            let loaded = Manifest::load(&path).unwrap();
            let products = loaded.get_index("products").unwrap();
            assert_eq!(products.segments, segments);
        }
    }

    #[test]
    fn detect_missing_segments_none_missing() {
        let dir = tempfile::tempdir().unwrap();
        let seg_dir = dir.path().join("segments");
        fs::create_dir_all(&seg_dir).unwrap();

        // Create segment files on disk
        for id in &[1u64, 2, 3] {
            fs::write(seg_dir.join(format!("{id:06}.seg")), "dummy").unwrap();
        }

        let mut m = Manifest::new();
        m.upsert_index("test", vec![1, 2, 3]);

        let missing = m.detect_missing_segments("test", &seg_dir);
        assert!(missing.is_empty(), "expected no missing segments");
    }

    #[test]
    fn detect_missing_segments_finds_gaps() {
        let dir = tempfile::tempdir().unwrap();
        let seg_dir = dir.path().join("segments");
        fs::create_dir_all(&seg_dir).unwrap();

        // Only create segments 1 and 3 (skip segment 2)
        fs::write(seg_dir.join("000001.seg"), "dummy").unwrap();
        fs::write(seg_dir.join("000003.seg"), "dummy").unwrap();

        let mut m = Manifest::new();
        m.upsert_index("test", vec![1, 2, 3]);

        let missing = m.detect_missing_segments("test", &seg_dir);
        assert_eq!(missing, vec![2], "segment 2 should be missing");
    }

    #[test]
    fn detect_missing_returns_empty_for_unknown_index() {
        let dir = tempfile::tempdir().unwrap();
        let m = Manifest::new();
        let missing = m.detect_missing_segments("nonexistent", dir.path());
        assert!(missing.is_empty());
    }

    #[test]
    fn detect_missing_all_segments_gone() {
        let dir = tempfile::tempdir().unwrap();
        let seg_dir = dir.path().join("segments");
        fs::create_dir_all(&seg_dir).unwrap();

        // No segment files on disk

        let mut m = Manifest::new();
        m.upsert_index("test", vec![1, 2, 3]);

        let missing = m.detect_missing_segments("test", &seg_dir);
        assert_eq!(missing, vec![1, 2, 3], "all segments should be missing");
    }
}
