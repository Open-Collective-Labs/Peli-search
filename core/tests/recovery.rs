use std::collections::HashMap;

use pelisearch_core::document::Document;
use pelisearch_core::engine::SearchEngine;

fn make_doc(id: &str, title: &str) -> Document {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), serde_json::json!(title));
    Document::new(id, fields).unwrap()
}

/// Write documents, restart, verify they survived.
#[test]
fn restart_recovery_basic() {
    let dir = tempfile::tempdir().unwrap();

    // First session: create index, add documents, flush
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine.add_document("test", make_doc("doc1", "hello world")).unwrap();
        engine.add_document("test", make_doc("doc2", "goodbye world")).unwrap();
        engine.flush().unwrap();
    }

    // Second session: reopen and verify
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.list_indexes().contains(&"test".to_string()));
        assert!(engine.get_document("test", "doc1").is_ok());
        assert!(engine.get_document("test", "doc2").is_ok());
        let results = engine.search("test", "hello").unwrap();
        assert_eq!(results.len(), 1);
    }
}

/// Multiple restart cycles with growing data.
#[test]
fn restart_recovery_multiple_cycles() {
    let dir = tempfile::tempdir().unwrap();

    for i in 0..5 {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        if i == 0 {
            engine.create_index("test").unwrap();
        }
        let doc_id = format!("doc_{i}");
        engine
            .add_document("test", make_doc(&doc_id, &format!("title_{i}")))
            .unwrap();
        engine.flush().unwrap();
    }

    // Final verification
    let engine = SearchEngine::open(dir.path()).unwrap();
    for i in 0..5 {
        let doc_id = format!("doc_{i}");
        assert!(
            engine.get_document("test", &doc_id).is_ok(),
            "{doc_id} should survive multiple restarts"
        );
    }
}

/// Simulate a crash by dropping the engine without flush.
/// WAL replay should recover unflushed data.
#[test]
fn crash_simulation_without_flush() {
    let dir = tempfile::tempdir().unwrap();

    // Session: write data, but do NOT flush (simulating crash)
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine.add_document("test", make_doc("crash_doc", "this must survive")).unwrap();
        // Drop without flush
    }

    // Recover: WAL should replay the write
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.index_exists("test"));
        assert!(
            engine.get_document("test", "crash_doc").is_ok(),
            "document should survive crash via WAL replay"
        );
    }
}

/// Crash during multi-document write, then verify partial recovery.
#[test]
fn crash_simulation_multiple_documents() {
    let dir = tempfile::tempdir().unwrap();

    // Session 1: flush baseline
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine.add_document("test", make_doc("base", "baseline doc")).unwrap();
        engine.flush().unwrap();
    }

    // Session 2: add docs without flush, then crash
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.add_document("test", make_doc("added_1", "added one")).unwrap();
        engine.add_document("test", make_doc("added_2", "added two")).unwrap();
        // Drop without flush
    }

    // Session 3: recover — both base docs and WAL-replayed docs should exist
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.get_document("test", "base").is_ok(), "baseline must survive");
        assert!(engine.get_document("test", "added_1").is_ok(), "WAL-replayed doc must survive");
        assert!(engine.get_document("test", "added_2").is_ok(), "WAL-replayed doc must survive");
    }
}

/// Crash after flush, then restart.
#[test]
fn crash_after_flush() {
    let dir = tempfile::tempdir().unwrap();

    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine.add_document("test", make_doc("safe", "flushed doc")).unwrap();
        engine.flush().unwrap();
    }

    // Simulate a second session that crashes after writing to WAL
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.add_document("test", make_doc("unsafe", "unflushed doc")).unwrap();
        // No flush — crash on drop
    }

    // Both docs should be present (WAL replays the unsafe write)
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.get_document("test", "safe").is_ok());
        assert!(engine.get_document("test", "unsafe").is_ok());
    }
}

/// Delete documents, crash, verify deletion is replayed.
#[test]
fn crash_with_deletions() {
    let dir = tempfile::tempdir().unwrap();

    // Session 1: add docs and flush
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine.add_document("test", make_doc("keep", "keep me")).unwrap();
        engine.add_document("test", make_doc("remove", "remove me")).unwrap();
        engine.flush().unwrap();
    }

    // Session 2: delete a doc without flush, then crash
    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.remove_document("test", "remove").unwrap();
        // Crash without flush
    }

    // Session 3: verify deletion was replayed from WAL
    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert!(engine.get_document("test", "keep").is_ok());
        assert!(
            engine.get_document("test", "remove").is_err(),
            "deleted document should be gone after WAL replay"
        );
    }
}

/// Multiple indexes, crash, verify each index independently.
#[test]
fn crash_multi_index_recovery() {
    let dir = tempfile::tempdir().unwrap();

    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("alpha").unwrap();
        engine.create_index("beta").unwrap();
        engine.add_document("alpha", make_doc("a1", "alpha doc")).unwrap();
        engine.add_document("beta", make_doc("b1", "beta doc")).unwrap();
        engine.flush().unwrap();
    }

    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.add_document("alpha", make_doc("a2", "alpha second")).unwrap();
        engine.add_document("beta", make_doc("b2", "beta second")).unwrap();
        // Crash
    }

    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        assert_eq!(engine.list_indexes(), vec!["alpha", "beta"]);
        let alpha = engine.search("alpha", "alpha").unwrap();
        assert_eq!(alpha.len(), 2, "both alpha docs recovered");
        let beta = engine.search("beta", "beta").unwrap();
        assert_eq!(beta.len(), 2, "both beta docs recovered");
    }
}

/// Search still works correctly after crash recovery.
#[test]
fn search_after_crash_recovery() {
    let dir = tempfile::tempdir().unwrap();

    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine.create_index("test").unwrap();
        engine
            .add_document("test", make_doc("doc1", "electric bike"))
            .unwrap();
        engine
            .add_document("test", make_doc("doc2", "mountain bike"))
            .unwrap();
        engine.flush().unwrap();
    }

    {
        let mut engine = SearchEngine::open(dir.path()).unwrap();
        engine
            .add_document("test", make_doc("doc3", "road bike"))
            .unwrap();
        // Crash
    }

    {
        let engine = SearchEngine::open(dir.path()).unwrap();
        let results = engine.search("test", "bike").unwrap();
        assert_eq!(results.len(), 3, "all bike docs found after crash");
    }
}
