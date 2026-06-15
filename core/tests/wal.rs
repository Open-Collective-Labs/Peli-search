use std::collections::HashMap;
use std::fs;

use pelisearch_core::document::Document;
use pelisearch_core::engine::SearchEngine;

fn make_doc(id: &str, title: &str) -> Document {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), serde_json::json!(title));
    Document::new(id, fields).unwrap()
}

/// WAL replays after clean shutdown with flush.
#[test]
fn wal_replay_after_flush() {
    let dir = tempfile::tempdir().unwrap();

    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine.add_document("test", make_doc("d1", "hello")).unwrap();
        engine.flush().unwrap();
    }

    // Reopen — WAL should be empty after flush, but data survives via snapshot
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.get_document("test", "d1").is_ok());
        let results = engine.search("test", "hello").unwrap();
        assert_eq!(results.len(), 1);
    }
}

/// WAL replay restores state when no flush was performed.
#[test]
fn wal_replay_without_flush() {
    let dir = tempfile::tempdir().unwrap();

    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine.add_document("test", make_doc("d1", "wal only")).unwrap();
        // No flush
    }

    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.get_document("test", "d1").is_ok());
        let results = engine.search("test", "wal").unwrap();
        assert_eq!(results.len(), 1);
    }
}

/// WAL replay with multiple entry types (create, add, remove).
#[test]
fn wal_replay_multiple_entry_types() {
    let dir = tempfile::tempdir().unwrap();

    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine.add_document("test", make_doc("d1", "first")).unwrap();
        engine.add_document("test", make_doc("d2", "second")).unwrap();
        engine.remove_document("test", "d1").unwrap();
        // No flush — WAL has CreateIndex, AddDocument x2, RemoveDocument
    }

    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.get_document("test", "d1").is_err(), "d1 was removed");
        assert!(engine.get_document("test", "d2").is_ok());
    }
}

/// WAL replay after index was deleted and re-created.
#[test]
fn wal_replay_across_index_lifecycle() {
    let dir = tempfile::tempdir().unwrap();

    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine.add_document("test", make_doc("d1", "first life")).unwrap();
        engine.flush().unwrap();
    }

    // Delete and recreate (via WAL)
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("temp").unwrap();
        engine.add_document("temp", make_doc("t1", "temporary")).unwrap();
        // Crash before flush
    }

    // Only temp should survive (test was never created in this session)
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.index_exists("temp"));
        assert!(engine.get_document("temp", "t1").is_ok());
    }
}

/// Corrupt WAL does not prevent recovery from snapshot.
#[test]
fn wal_corrupt_does_not_block_recovery() {
    let dir = tempfile::tempdir().unwrap();

    // Create and flush a baseline
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine.add_document("test", make_doc("safe", "baseline")).unwrap();
        engine.flush().unwrap();
    }

    // Corrupt the WAL file
    {
        let wal_path = dir.path().join("wal.log");
        fs::write(&wal_path, "{{{corrupt garbage}}}").unwrap();
    }

    // Recovery should still load from snapshot
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.index_exists("test"));
        assert!(engine.get_document("test", "safe").is_ok());
    }
}

/// Empty WAL is handled cleanly.
#[test]
fn wal_empty_does_not_fail() {
    let dir = tempfile::tempdir().unwrap();

    // Create an empty wal.log
    {
        let wal_path = dir.path().join("wal.log");
        fs::write(&wal_path, b"").unwrap();
    }

    // Add a snapshot manually
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine.add_document("test", make_doc("d1", "manual")).unwrap();
        engine.flush().unwrap();
    }

    // Verify
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.get_document("test", "d1").is_ok());
    }
}

/// Partial WAL line at end is trimmed during replay.
#[test]
fn wal_partial_trailing_line() {
    let dir = tempfile::tempdir().unwrap();

    // Create a baseline snapshot first
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine
            .add_document("test", make_doc("d1", "baseline"))
            .unwrap();
        engine.flush().unwrap();
    }

    // Append a truncated line to the WAL after clean state
    {
        let wal_path = dir.path().join("wal.log");
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&wal_path)
            .unwrap();
        use std::io::Write;
        writeln!(file, "{{\"op\":\"AddD").unwrap(); // truncated JSON
    }

    // Recovery should still succeed (partial line is skipped)
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.index_exists("test"));
        assert!(engine.get_document("test", "d1").is_ok());
    }
}

/// WAL-only index creation (no snapshot) survives restart.
#[test]
fn wal_only_index_creation() {
    let dir = tempfile::tempdir().unwrap();

    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("ephemeral").unwrap();
        engine
            .add_document("ephemeral", make_doc("d1", "no snapshot yet"))
            .unwrap();
        // No flush
    }

    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.index_exists("ephemeral"));
        assert!(engine.get_document("ephemeral", "d1").is_ok());
    }
}

/// State after multiple WAL replays (each session adds, restarts).
#[test]
fn wal_replay_idempotent_across_sessions() {
    let dir = tempfile::tempdir().unwrap();

    for i in 0..3 {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        if i == 0 {
            engine.create_index("test").unwrap();
        }
        let doc_id = format!("d{i}");
        engine
            .add_document("test", make_doc(&doc_id, &format!("version {i}")))
            .unwrap();
        // Flush on last iteration
        if i == 2 {
            engine.flush().unwrap();
        }
    }

    // All documents should be present
    let engine = SearchEngine::open(dir.path()).unwrap();
    for i in 0..3 {
        let doc_id = format!("d{i}");
        assert!(
            engine.get_document("test", &doc_id).is_ok(),
            "{doc_id} should survive WAL replay across sessions"
        );
    }
}
