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

/// Merge a specific subset of segments into a single compact segment.
///
/// Unlike [`compact_index`] which merges ALL segments, this function
/// merges only the segments identified by `segment_ids`. Segments not
/// in the list are left untouched.
///
/// # Flow
///
/// 1. Load each selected segment from disk.
/// 2. Merge documents (oldest segment wins for duplicates).
/// 3. Replay WAL entries on top.
/// 4. Write a new compact segment with the next available ID.
/// 5. Delete the old selected segment files.
///
/// # Arguments
///
/// * `index_dir` - Directory containing the segment files
/// * `index_name` - Name of the index (for WAL filtering)
/// * `segment_ids` - IDs of segments to merge (must have at least 2)
/// * `wal_entries` - WAL entries to apply after segment merge
///
/// # Returns
///
/// The ID of the newly created compact segment.
///
/// # Errors
///
/// Returns an I/O error if any segment cannot be read, written, or deleted.
pub fn compact_segments(
    index_dir: &Path,
    index_name: &str,
    segment_ids: &[u64],
    wal_entries: &[WalEntry],
) -> io::Result<u64> {
    if segment_ids.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "compact_segments requires at least 2 segment IDs, got {}",
                segment_ids.len()
            ),
        ));
    }

    // Discover all segments and filter to the selected ones
    let all_segments = Segment::discover(index_dir)?;
    let selected: Vec<&Segment> = all_segments
        .iter()
        .filter(|s| segment_ids.contains(&s.id))
        .collect();

    if selected.len() != segment_ids.len() {
        let found: Vec<u64> = selected.iter().map(|s| s.id).collect();
        let missing: Vec<u64> = segment_ids
            .iter()
            .filter(|id| !found.contains(id))
            .copied()
            .collect();
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("segments not found on disk: {missing:?}"),
        ));
    }

    // Determine the next available ID (must not collide with any existing)
    let next_id = all_segments.last().map(|s| s.id + 1).unwrap_or(1);

    // Build the merged index from selected segments only
    let mapping = if let Some(newest) = selected.last() {
        let data = Segment::read_segment(&newest.path)?;
        data.mapping
    } else {
        Mapping::new(vec![])
    };

    let mut index = Index::new(index_name, mapping);

    // Load from selected segments, oldest first
    for seg in &selected {
        let data = Segment::read_segment(&seg.path)?;
        for (doc_id, doc) in &data.documents {
            if index.get_document(doc_id).is_err() {
                let _ = index.add_document(doc.clone());
            }
        }
    }

    // Apply WAL entries on top
    for entry in wal_entries {
        match entry {
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

    index.rebuild_stats();

    // Extract segment data
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

    // Write the compact segment
    let seg = Segment::write_segment(index_dir, next_id, &data)?;

    // Delete only the selected old segments
    for old_seg in &selected {
        Segment::delete_segment(&old_seg.path)?;
    }

    Ok(seg.id)
}

/// Build an `Index` from all segments (in order, oldest first), then apply
/// WAL entries on top. If no segments exist, starts from an empty index.
///
/// When the same document ID appears in multiple segments, the *oldest*
/// segment's version wins (first-seen semantics), because segments are
/// immutable and once a document is written it cannot change — updates
/// create new documents in newer segments (or tombstone via WAL).
fn build_merged_index(
    old_segments: &[Segment],
    index_name: &str,
    wal_entries: &[WalEntry],
) -> io::Result<Index> {
    // Determine the mapping — use the newest segment's mapping (may have
    // the most up-to-date schema), or default.
    let mapping = if let Some(newest) = old_segments.last() {
        let data = Segment::read_segment(&newest.path)?;
        data.mapping
    } else {
        Mapping::new(vec![])
    };

    let mut index = Index::new(index_name, mapping.clone());

    // Load documents from ALL segments, oldest first. Segments are
    // immutable, so the first occurrence of a document ID wins (oldest
    // segment takes precedence).
    for seg in old_segments {
        let data = Segment::read_segment(&seg.path)?;
        for (doc_id, doc) in &data.documents {
            if index.get_document(doc_id).is_err() {
                let _ = index.add_document(doc.clone());
            }
        }
    }

    // Apply WAL entries on top (newer than any segment)
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
    fn compact_merges_all_segments_not_just_newest() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        // Segment 1: doc1, doc2
        let mut data1 = sample_segment_data("test");
        data1.documents
            .insert("doc2".to_string(), make_doc("doc2", "from_seg1"));
        Segment::write_segment(&idx_dir, 1, &data1).unwrap();

        // Segment 2: doc3 (only in seg2)
        let mut data2 = sample_segment_data("test");
        data2.documents
            .insert("doc3".to_string(), make_doc("doc3", "from_seg2"));
        Segment::write_segment(&idx_dir, 2, &data2).unwrap();

        // Compact — must preserve doc1 (from both), doc2 (from seg1), doc3 (from seg2)
        let _new_id = compact_index(&idx_dir, "test", &[]).unwrap();

        let remaining = Segment::discover(&idx_dir).unwrap();
        assert_eq!(remaining.len(), 1);
        let loaded = Segment::read_segment(&remaining[0].path).unwrap();
        assert_eq!(loaded.documents.len(), 3, "all 3 docs from both segments must be preserved");
        assert!(loaded.documents.contains_key("doc1"));
        assert!(loaded.documents.contains_key("doc2"));
        assert!(loaded.documents.contains_key("doc3"));
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

    // --- compact_segments tests ---

    #[test]
    fn compact_segments_merges_selected_subset() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        // Write 4 segments
        let mut data1 = sample_segment_data("test");
        data1.documents.insert("doc1".to_string(), make_doc("doc1", "alpha"));
        Segment::write_segment(&idx_dir, 1, &data1).unwrap();

        let mut data2 = sample_segment_data("test");
        data2.documents.insert("doc2".to_string(), make_doc("doc2", "bravo"));
        Segment::write_segment(&idx_dir, 2, &data2).unwrap();

        let mut data3 = sample_segment_data("test");
        data3.documents.insert("doc3".to_string(), make_doc("doc3", "charlie"));
        Segment::write_segment(&idx_dir, 3, &data3).unwrap();

        let mut data4 = sample_segment_data("test");
        data4.documents.insert("doc4".to_string(), make_doc("doc4", "delta"));
        Segment::write_segment(&idx_dir, 4, &data4).unwrap();

        // Merge only segments 1 and 2 (not 3 and 4)
        let new_id = compact_segments(&idx_dir, "test", &[1, 2], &[]).unwrap();

        // 4 original - 2 deleted + 1 new = 3 segments remaining
        let remaining = Segment::discover(&idx_dir).unwrap();
        assert_eq!(remaining.len(), 3);
        let ids: Vec<u64> = remaining.iter().map(|s| s.id).collect();
        assert!(ids.contains(&3), "segment 3 should be untouched");
        assert!(ids.contains(&4), "segment 4 should be untouched");
        assert!(ids.contains(&new_id), "new compact segment should exist");

        // The compact segment should have doc1 + doc2 + doc3 (from seg1 original doc1)
        let compact_seg = remaining.iter().find(|s| s.id == new_id).unwrap();
        let loaded = Segment::read_segment(&compact_seg.path).unwrap();
        assert!(loaded.documents.contains_key("doc1"));
        assert!(loaded.documents.contains_key("doc2"));
    }

    #[test]
    fn compact_segments_preserves_unselected_segments() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        let data1 = sample_segment_data("test");
        Segment::write_segment(&idx_dir, 1, &data1).unwrap();

        let mut data2 = sample_segment_data("test");
        data2.documents.insert("doc2".to_string(), make_doc("doc2", "only in seg2"));
        Segment::write_segment(&idx_dir, 2, &data2).unwrap();

        let mut data3 = sample_segment_data("test");
        data3.documents.insert("doc3".to_string(), make_doc("doc3", "only in seg3"));
        Segment::write_segment(&idx_dir, 3, &data3).unwrap();

        // Merge only segments 1 and 3
        let _new_id = compact_segments(&idx_dir, "test", &[1, 3], &[]).unwrap();

        let remaining = Segment::discover(&idx_dir).unwrap();
        // Segments: 2 (untouched), _new_id (merged 1+3)
        assert_eq!(remaining.len(), 2);

        // Segment 2 must still exist and be readable
        let seg2_path = Segment::discover(&idx_dir)
            .unwrap()
            .into_iter()
            .find(|s| s.id == 2)
            .unwrap();
        let seg2_data = Segment::read_segment(&seg2_path.path).unwrap();
        assert!(seg2_data.documents.contains_key("doc2"));
    }

    #[test]
    fn compact_segments_requires_at_least_two() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        let data = sample_segment_data("test");
        Segment::write_segment(&idx_dir, 1, &data).unwrap();

        let err = compact_segments(&idx_dir, "test", &[1], &[]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(format!("{}", err).contains("at least 2"));
    }

    #[test]
    fn compact_segments_fails_on_missing_id() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        let data = sample_segment_data("test");
        Segment::write_segment(&idx_dir, 1, &data).unwrap();

        let err = compact_segments(&idx_dir, "test", &[1, 99], &[]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(format!("{}", err).contains("99"));
    }

    #[test]
    fn compact_segments_applies_wal_entries() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        let data1 = sample_segment_data("test");
        Segment::write_segment(&idx_dir, 1, &data1).unwrap();

        let mut data2 = sample_segment_data("test");
        data2.documents.insert("doc2".to_string(), make_doc("doc2", "from seg2"));
        Segment::write_segment(&idx_dir, 2, &data2).unwrap();

        // WAL adds doc3 and removes doc1
        let entries = vec![
            WalEntry::AddDocument {
                index_name: "test".into(),
                document: make_doc("doc3", "from wal"),
            },
            WalEntry::RemoveDocument {
                index_name: "test".into(),
                doc_id: "doc1".into(),
            },
        ];

        let _ = compact_segments(&idx_dir, "test", &[1, 2], &entries).unwrap();

        let remaining = Segment::discover(&idx_dir).unwrap();
        assert_eq!(remaining.len(), 1);
        let loaded = Segment::read_segment(&remaining[0].path).unwrap();
        assert!(!loaded.documents.contains_key("doc1"), "doc1 removed via WAL");
        assert!(loaded.documents.contains_key("doc2"), "doc2 from seg2 preserved");
        assert!(loaded.documents.contains_key("doc3"), "doc3 added via WAL");
    }

    #[test]
    fn compact_segments_deduplicates_documents() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        // Both segments contain doc1
        let data1 = sample_segment_data("test");
        Segment::write_segment(&idx_dir, 1, &data1).unwrap();

        let mut data2 = sample_segment_data("test");
        data2.documents.insert("doc1".to_string(), make_doc("doc1", "version2"));
        Segment::write_segment(&idx_dir, 2, &data2).unwrap();

        let new_id = compact_segments(&idx_dir, "test", &[1, 2], &[]).unwrap();
        let remaining = Segment::discover(&idx_dir).unwrap();
        let compact = remaining.iter().find(|s| s.id == new_id).unwrap();
        let loaded = Segment::read_segment(&compact.path).unwrap();

        // doc1 should appear only once (first-wins from segment 1)
        assert_eq!(loaded.documents.len(), 1);
        assert!(loaded.documents.contains_key("doc1"));
    }

    #[test]
    fn compact_segments_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("indexes").join("test");
        fs::create_dir_all(&idx_dir).unwrap();

        let mut data1 = sample_segment_data("test");
        data1.documents.insert("doc1".to_string(), make_doc("doc1", "hello"));
        Segment::write_segment(&idx_dir, 1, &data1).unwrap();

        let mut data2 = sample_segment_data("test");
        data2.documents.insert("doc2".to_string(), make_doc("doc2", "world"));
        Segment::write_segment(&idx_dir, 2, &data2).unwrap();

        let _ = compact_segments(&idx_dir, "test", &[1, 2], &[]).unwrap();

        // Simulate restart: discover and load
        let segments = Segment::discover(&idx_dir).unwrap();
        assert_eq!(segments.len(), 1);
        let loaded = Segment::read_segment(&segments[0].path).unwrap();
        assert_eq!(loaded.documents.len(), 2);
        assert!(loaded.documents.contains_key("doc1"));
        assert!(loaded.documents.contains_key("doc2"));
    }
}
