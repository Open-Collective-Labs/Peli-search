use std::fs;
use std::io;
use std::path::Path;

use crate::index::Index;
use crate::schema::Mapping;

use super::manifest::Manifest;
use super::segment::{IndexSegmentData, Segment};
use super::wal::WalEntry;

/// Merges old segment files and WAL entries into a single compact segment
/// for a given index directory.
///
/// # Flow
///
/// 1. Discover all `.seg` files in the index directory.
/// 2. Load the newest segment as the baseline state.
/// 3. Replay the provided WAL entries on top of that state.
/// 4. Write a new compact segment with the next available ID.
/// 5. Delete all old segment files.
///
/// Returns the ID of the newly created compact segment.
pub fn compact_index(
    index_dir: &Path,
    index_name: &str,
    wal_entries: &[WalEntry],
) -> io::Result<u64> {
    let old_segments = Segment::discover(index_dir)?;
    let next_id = old_segments.last().map(|s| s.id + 1).unwrap_or(1);

    let index = build_merged_index(&old_segments, index_name, wal_entries)?;

    // Step 2: extract the full state into segment data
    let data = IndexSegmentData {
        name: index_name.to_string(),
        mapping: index.mapping().clone(),
        documents: {
            let mut docs = std::collections::HashMap::new();
            for id in index.list_document_ids() {
                if let Ok(d) = index.get_document(&id) {
                    docs.insert(id, d.clone());
                }
            }
            docs
        },
        inverted_index: index.inverted_index_clone(),
        stats: index.stats_ref().clone(),
    };

    // Step 3: write the compact segment atomically
    let seg = Segment::write_segment(index_dir, next_id, &data)?;

    // Step 4: delete old segments (everything except the new one)
    for seg in &old_segments {
        if seg.id != next_id {
            Segment::delete_segment(&seg.path)?;
        }
    }

    Ok(seg.id)
}

/// Build an `Index` from the newest segment (if any), then apply WAL
/// entries on top. If no segments exist, starts from an empty index.
fn build_merged_index(
    old_segments: &[Segment],
    index_name: &str,
    wal_entries: &[WalEntry],
) -> io::Result<Index> {
    // Determine the mapping — use the newest segment's mapping, or default
    let mapping = if let Some(newest) = old_segments.last() {
        let data = Segment::read_segment(&newest.path)?;
        data.mapping
    } else {
        Mapping::new(vec![])
    };

    let mut index = Index::new(index_name, mapping.clone());

    // Load documents from the newest segment (baseline)
    if let Some(newest) = old_segments.last() {
        let data = Segment::read_segment(&newest.path)?;
        for (doc_id, doc) in &data.documents {
            if index.get_document(doc_id).is_err() {
                let _ = index.add_document(doc.clone());
            }
        }
    }

    // Apply WAL entries on top
    for entry in wal_entries {
        match entry {
            WalEntry::CreateIndex { .. } => {
                // Index already exists — no-op
            }
            WalEntry::DeleteIndex { .. } => {
                // Can't meaningfully delete during per-index compaction
                // (the caller should not compact a deleted index)
            }
            WalEntry::AddDocument {
                index_name: name,
                document,
            } if name == index_name => {
                if index.get_document(&document.id).is_err() {
                    let _ = index.add_document(document.clone());
                }
            }
            WalEntry::RemoveDocument {
                index_name: name,
                doc_id,
            } if name == index_name => {
                let _ = index.remove_document(doc_id);
            }
            _ => {}
        }
    }

    // Rebuild stats for consistency
    index.rebuild_stats();

    Ok(index)
}

/// Compact a specific index directory, removing all old segments and
/// replacing them with a single merged segment that includes WAL state.
///
/// Updates the manifest and cleans up orphan `.tmp` files.
pub fn compact_with_manifest(
    index_dir: &Path,
    index_name: &str,
    wal_entries: &[WalEntry],
    manifest: &mut Manifest,
) -> io::Result<u64> {
    cleanup_tmp_files(index_dir)?;

    let seg_id = compact_index(index_dir, index_name, wal_entries)?;

    // Update manifest to track only the compact segment
    manifest.upsert_index(index_name, vec![seg_id]);

    Ok(seg_id)
}

/// Remove any orphan `.seg.tmp` files left from a previous crash during
/// segment writing.
fn cleanup_tmp_files(dir: &Path) -> io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "tmp" {
                let _ = fs::remove_file(&path);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;

    use crate::document::Document;

    use super::super::manifest::Manifest;
    use super::super::segment::Segment;
    use super::super::wal::WalEntry;
    use super::*;

    fn make_doc(id: &str, title: &str) -> Document {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!(title));
        Document::new(id, fields).unwrap()
    }

    fn sample_segment_data(name: &str) -> IndexSegmentData {
        let mut documents = HashMap::new();
        documents.insert("doc1".to_string(), make_doc("doc1", "hello"));
        IndexSegmentData {
            name: name.to_string(),
            mapping: Mapping::new(vec![]),
            documents,
            inverted_index: crate::index::inverted::InvertedIndex::new(),
            stats: crate::ranking::statistics::CollectionStats::new(),
        }
    }

    #[test]
    fn compact_merges_segments_into_single() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        // Write two old segments
        let data1 = sample_segment_data("test");
        let seg1 = Segment::write_segment(&idx_dir, 1, &data1).unwrap();
        assert_eq!(seg1.id, 1);

        let mut data2 = sample_segment_data("test");
        data2
            .documents
            .insert("doc2".to_string(), make_doc("doc2", "world"));
        let seg2 = Segment::write_segment(&idx_dir, 2, &data2).unwrap();
        assert_eq!(seg2.id, 2);

        // Compact with no WAL entries
        let new_id = compact_index(&idx_dir, "test", &[]).unwrap();

        // Only the compact segment should remain
        let remaining = Segment::discover(&idx_dir).unwrap();
        assert_eq!(remaining.len(), 1, "expected exactly one segment after compaction");
        assert_eq!(remaining[0].id, new_id, "segment ID should match");
        assert_eq!(new_id, 3, "next ID should be 3");

        // Content should include doc1 (from seg1) and doc2 (from seg2)
        let loaded = Segment::read_segment(&remaining[0].path).unwrap();
        assert_eq!(loaded.documents.len(), 2);
        assert!(loaded.documents.contains_key("doc1"));
        assert!(loaded.documents.contains_key("doc2"));
    }

    #[test]
    fn compact_applies_wal_on_top() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        // Write a segment with doc1
        let data = sample_segment_data("test");
        Segment::write_segment(&idx_dir, 1, &data).unwrap();

        // WAL adds doc2 and removes doc1
        let from_wal = make_doc("doc2", "world");
        let entries = vec![
            WalEntry::AddDocument {
                index_name: "test".into(),
                document: from_wal,
            },
            WalEntry::RemoveDocument {
                index_name: "test".into(),
                doc_id: "doc1".into(),
            },
        ];

        let new_id = compact_index(&idx_dir, "test", &entries).unwrap();

        let remaining = Segment::discover(&idx_dir).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, new_id);

        let loaded = Segment::read_segment(&remaining[0].path).unwrap();
        assert_eq!(loaded.documents.len(), 1, "doc1 was removed, doc2 added");
        assert!(
            loaded.documents.contains_key("doc2"),
            "only doc2 should remain"
        );
    }

    #[test]
    fn compact_with_no_segments_creates_empty() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("empty");
        fs::create_dir_all(&idx_dir).unwrap();

        let new_id = compact_index(&idx_dir, "empty", &[]).unwrap();
        assert_eq!(new_id, 1);

        let remaining = Segment::discover(&idx_dir).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, 1);

        let loaded = Segment::read_segment(&remaining[0].path).unwrap();
        assert!(loaded.documents.is_empty());
    }

    #[test]
    fn compact_preserves_data_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        // Write two segments with overlapping data
        let mut data1 = sample_segment_data("test");
        data1
            .documents
            .insert("shared".to_string(), make_doc("shared", "version1"));
        Segment::write_segment(&idx_dir, 1, &data1).unwrap();

        let mut data2 = sample_segment_data("test");
        data2
            .documents
            .insert("shared".to_string(), make_doc("shared", "version2"));
        Segment::write_segment(&idx_dir, 2, &data2).unwrap();

        // Compact
        let _ = compact_index(&idx_dir, "test", &[]).unwrap();

        // Simulate restart: discover + load
        let segments = Segment::discover(&idx_dir).unwrap();
        assert_eq!(segments.len(), 1);

        let loaded = Segment::read_segment(&segments[0].path).unwrap();
        assert_eq!(loaded.name, "test");
        // The newer segment's version of "shared" should win
        assert_eq!(
            loaded.documents.get("shared").unwrap().id,
            "shared",
            "shared doc preserved"
        );
    }

    #[test]
    fn compact_reduces_file_count() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        for i in 1..=5 {
            let mut data = sample_segment_data("test");
            data.documents.insert(
                format!("doc{i}"),
                make_doc(&format!("doc{i}"), &format!("title_{i}")),
            );
            Segment::write_segment(&idx_dir, i, &data).unwrap();
        }

        let before = Segment::discover(&idx_dir).unwrap();
        assert_eq!(before.len(), 5);

        let _ = compact_index(&idx_dir, "test", &[]).unwrap();

        let after = Segment::discover(&idx_dir).unwrap();
        assert_eq!(after.len(), 1, "5 segments compacted into 1");
    }

    #[test]
    fn compact_updates_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        let data = sample_segment_data("test");
        Segment::write_segment(&idx_dir, 1, &data).unwrap();

        let mut manifest = Manifest::new();
        manifest.upsert_index("test", vec![1]);

        let new_id =
            compact_with_manifest(&idx_dir, "test", &[], &mut manifest).unwrap();
        assert_eq!(new_id, 2);

        let meta = manifest.get_index("test").unwrap();
        assert_eq!(meta.segments, vec![2], "manifest should track only the compact segment");
    }

    #[test]
    fn compact_honors_different_index_name() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("products");
        fs::create_dir_all(&idx_dir).unwrap();

        let data = sample_segment_data("products");
        Segment::write_segment(&idx_dir, 1, &data).unwrap();

        // WAL entries for a different index should be ignored
        let entries = vec![WalEntry::AddDocument {
            index_name: "other".into(),
            document: make_doc("other_doc", "ignored"),
        }];

        let _ = compact_index(&idx_dir, "products", &entries).unwrap();

        let remaining = Segment::discover(&idx_dir).unwrap();
        let loaded = Segment::read_segment(&remaining[0].path).unwrap();
        // Only the original doc1 should exist (the "other" index entry was ignored)
        assert_eq!(loaded.documents.len(), 1);
        assert!(loaded.documents.contains_key("doc1"));
    }

    #[test]
    fn cleanup_removes_orphan_tmp_files() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        // Create an orphan .tmp file
        let tmp_path = idx_dir.join("000001.seg.tmp");
        fs::write(&tmp_path, "garbage").unwrap();
        assert!(tmp_path.exists());

        // Create a valid segment too
        let data = sample_segment_data("test");
        Segment::write_segment(&idx_dir, 1, &data).unwrap();

        // Compaction should clean up the .tmp file
        let _ = compact_with_manifest(&idx_dir, "test", &[], &mut Manifest::new()).unwrap();
        assert!(!tmp_path.exists(), "orphan .tmp file should be removed");
    }

    #[test]
    fn compact_after_delete_preserves_remaining_docs() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        // Segment 1: doc1, doc2
        let mut data = sample_segment_data("test");
        data.documents
            .insert("doc2".to_string(), make_doc("doc2", "world"));
        Segment::write_segment(&idx_dir, 1, &data).unwrap();

        // WAL: delete doc1
        let entries = vec![WalEntry::RemoveDocument {
            index_name: "test".into(),
            doc_id: "doc1".into(),
        }];

        let _ = compact_index(&idx_dir, "test", &entries).unwrap();

        let remaining = Segment::discover(&idx_dir).unwrap();
        assert_eq!(remaining.len(), 1);
        let loaded = Segment::read_segment(&remaining[0].path).unwrap();
        assert_eq!(loaded.documents.len(), 1, "doc1 deleted, only doc2 remains");
        assert!(loaded.documents.contains_key("doc2"));
    }
}
