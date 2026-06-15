use std::collections::HashMap;
use std::fs;

use pelisearch_core::document::Document;
use pelisearch_core::engine::SearchEngine;

fn make_doc(id: &str, title: &str) -> Document {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), serde_json::json!(title));
    Document::new(id, fields).unwrap()
}

fn index_dir(base: &std::path::Path, name: &str) -> std::path::PathBuf {
    base.join("indexes").join(name)
}

/// Backup a snapshot, delete the index, restore snapshot, verify.
#[test]
fn snapshot_backup_restore() {
    let dir = tempfile::tempdir().unwrap();

    // Create data and flush
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine
            .add_document("test", make_doc("doc1", "backup me"))
            .unwrap();
        engine.flush().unwrap();
    }

    // Backup: copy the snapshot file to a safe location
    let snap_path = index_dir(dir.path(), "test").join("snapshot.json");
    assert!(snap_path.exists(), "snapshot must exist after flush");
    let backup_dir = dir.path().join("backup");
    fs::create_dir_all(&backup_dir).unwrap();
    let backup_path = backup_dir.join("snapshot_backup.json");
    fs::copy(&snap_path, &backup_path).unwrap();

    // Delete the index (simulate data loss)
    fs::remove_dir_all(index_dir(dir.path(), "test")).unwrap();

    // The index should be gone
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(!engine.index_exists("test"));
    }

    // Restore: copy the backup back
    fs::create_dir_all(index_dir(dir.path(), "test")).unwrap();
    fs::copy(&backup_path, index_dir(dir.path(), "test").join("snapshot.json")).unwrap();

    // Also need to update manifest to be aware of the index
    // We do this by opening the engine which will discover nothing (no manifest entry),
    // so we need to write the manifest entry manually.
    {
        use pelisearch_core::storage::Manifest;
        let mut manifest = Manifest::load(dir.path().join("manifest.json")).unwrap();
        manifest.upsert_index("test", vec![]);
        manifest.save(dir.path().join("manifest.json")).unwrap();
    }

    // Verify restored data
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(
            engine.index_exists("test"),
            "index must exist after snapshot restore"
        );
        assert!(
            engine.get_document("test", "doc1").is_ok(),
            "document must be restored from snapshot"
        );
    }
}

/// Multiple snapshots across sessions, restore to an earlier point.
#[test]
fn snapshot_point_in_time_restore() {
    let dir = tempfile::tempdir().unwrap();

    // Session 1: baseline
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine
            .add_document("test", make_doc("v1", "version one"))
            .unwrap();
        engine.flush().unwrap();
    }

    // Backup the first snapshot
    let backup_v1 = {
        let snap = index_dir(dir.path(), "test").join("snapshot.json");
        let backup = dir.path().join("snapshot_v1.json");
        fs::copy(&snap, &backup).unwrap();
        backup
    };

    // Session 2: add more docs
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine
            .add_document("test", make_doc("v2", "version two"))
            .unwrap();
        engine.flush().unwrap();
    }

    // Now both version docs exist
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.get_document("test", "v1").is_ok());
        assert!(engine.get_document("test", "v2").is_ok());
    }

    // Restore to point-in-time v1: delete current snapshot, replace with v1
    fs::remove_dir_all(index_dir(dir.path(), "test")).unwrap();
    fs::create_dir_all(index_dir(dir.path(), "test")).unwrap();
    fs::copy(&backup_v1, index_dir(dir.path(), "test").join("snapshot.json")).unwrap();

    // The snapshot only has v1 — after recovery, v2 should be gone
    {
        use pelisearch_core::storage::Manifest;
        let mut manifest = Manifest::load(dir.path().join("manifest.json")).unwrap();
        manifest.upsert_index("test", vec![]);
        manifest.save(dir.path().join("manifest.json")).unwrap();
    }

    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.get_document("test", "v1").is_ok());
        assert!(
            engine.get_document("test", "v2").is_err(),
            "v2 should not exist after point-in-time restore"
        );
    }
}

/// Snapshot integrity check detects corruption.
#[test]
fn snapshot_corruption_detected() {
    let dir = tempfile::tempdir().unwrap();

    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine
            .add_document("test", make_doc("doc1", "hello"))
            .unwrap();
        engine.flush().unwrap();
    }

    // Corrupt the snapshot file
    let snap_path = index_dir(dir.path(), "test").join("snapshot.json");
    let content = fs::read_to_string(&snap_path).unwrap();
    // Replace a value to corrupt the checksum
    let corrupted = content.replace("\"hello\"", "\"corrupted\"");
    fs::write(&snap_path, &corrupted).unwrap();

    // Opening the engine should fail for this index
    // (the storage layer returns InvalidData for checksum mismatch)
    let engine = SearchEngine::open(dir.path());
    assert!(
        engine.is_err(),
        "corrupt snapshot should cause open to fail"
    );
    let err = engine.unwrap_err().to_string();
    assert!(
        err.contains("integrity check failed") || err.contains("snapshot"),
        "error should mention snapshot corruption: {err}"
    );
}

/// Backup a snapshot to a different directory, delete original, restore.
#[test]
fn snapshot_restore_from_external_backup() {
    let dir = tempfile::tempdir().unwrap();
    let backup_dir = tempfile::tempdir().unwrap();

    // Create and flush
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine
            .add_document("test", make_doc("d1", "backup test"))
            .unwrap();
        engine.flush().unwrap();
    }

    // Copy snapshot to external backup location
    let snap_path = index_dir(dir.path(), "test").join("snapshot.json");
    let external_backup = backup_dir.path().join("snapshot.json");
    fs::copy(&snap_path, &external_backup).unwrap();

    // Wipe the entire storage directory
    fs::remove_dir_all(dir.path()).unwrap();
    fs::create_dir_all(index_dir(dir.path(), "test")).unwrap();

    // Restore: copy snapshot back and write manifest
    fs::copy(
        &external_backup,
        index_dir(dir.path(), "test").join("snapshot.json"),
    )
    .unwrap();

    {
        use pelisearch_core::storage::Manifest;
        let mut manifest = Manifest::new();
        manifest.upsert_index("test", vec![]);
        manifest.save(dir.path().join("manifest.json")).unwrap();
    }

    // Verify
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.get_document("test", "d1").is_ok());
        let results = engine.search("test", "backup").unwrap();
        assert_eq!(results.len(), 1);
    }
}

/// Multiple indexes backed up and restored independently.
#[test]
fn snapshot_restore_multi_index() {
    let dir = tempfile::tempdir().unwrap();

    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("alpha").unwrap();
        engine.create_index("beta").unwrap();
        engine
            .add_document("alpha", make_doc("a1", "alpha doc"))
            .unwrap();
        engine
            .add_document("beta", make_doc("b1", "beta doc"))
            .unwrap();
        engine.flush().unwrap();
    }

    // Back up both snapshots
    let backup = tempfile::tempdir().unwrap();
    for name in &["alpha", "beta"] {
        let src = index_dir(dir.path(), name).join("snapshot.json");
        let dst = backup.path().join(format!("{name}_snapshot.json"));
        fs::copy(&src, &dst).unwrap();
    }

    // Delete beta's data
    fs::remove_dir_all(index_dir(dir.path(), "beta")).unwrap();

    // Restore beta from backup
    fs::create_dir_all(index_dir(dir.path(), "beta")).unwrap();
    fs::copy(
        &backup.path().join("beta_snapshot.json"),
        index_dir(dir.path(), "beta").join("snapshot.json"),
    )
    .unwrap();

    {
        use pelisearch_core::storage::Manifest;
        let mut manifest = Manifest::load(dir.path().join("manifest.json")).unwrap();
        manifest.upsert_index("beta", vec![]);
        manifest.save(dir.path().join("manifest.json")).unwrap();
    }

    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.get_document("alpha", "a1").is_ok(), "alpha should be intact");
        assert!(engine.get_document("beta", "b1").is_ok(), "beta should be restored");
    }
}

/// Restore a snapshot into a fresh directory.
#[test]
fn snapshot_migrate_to_new_directory() {
    let src_dir = tempfile::tempdir().unwrap();
    let dst_dir = tempfile::tempdir().unwrap();

    // Create data in source
    {
        let mut engine = SearchEngine::open(src_dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine
            .add_document("test", make_doc("d1", "migrated doc"))
            .unwrap();
        engine.flush().unwrap();
    }

    // Copy the snapshot to the new directory
    let src_snap = index_dir(src_dir.path(), "test").join("snapshot.json");
    let dst_index_dir = index_dir(dst_dir.path(), "test");
    fs::create_dir_all(&dst_index_dir).unwrap();
    fs::copy(&src_snap, dst_index_dir.join("snapshot.json")).unwrap();

    // Write manifest in the new directory
    {
        use pelisearch_core::storage::Manifest;
        let mut manifest = Manifest::new();
        manifest.upsert_index("test", vec![]);
        manifest.save(dst_dir.path().join("manifest.json")).unwrap();
    }

    // Open in new directory and verify
    {
        let engine = SearchEngine::open(dst_dir.path()).unwrap();
        assert!(engine.index_exists("test"));
        assert!(engine.get_document("test", "d1").is_ok());
        let results = engine.search("test", "migrated").unwrap();
        assert_eq!(results.len(), 1);
    }
}
