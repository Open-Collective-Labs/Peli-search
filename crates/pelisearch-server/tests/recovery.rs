mod common;

use common::{start_server, stop_server, url};
use serde_json::Value;

/// Test that data persists across server restarts.
#[test]
fn test_recovery_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();

    // ---- FIRST SESSION ----
    let (child, port) = start_server(data_dir);
    let client = reqwest::blocking::Client::new();

    // Create an index
    let resp = client
        .post(url(port, "/indexes"))
        .json(&serde_json::json!({"name": "recipes"}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 201);

    // Add documents
    for (id, title) in &[("r1", "Pancakes"), ("r2", "Omelette"), ("r3", "Soup")] {
        let resp = client
            .post(url(port, "/indexes/recipes/documents"))
            .json(&serde_json::json!({
                "id": id,
                "fields": {"title": title}
            }))
            .send()
            .unwrap();
        assert_eq!(resp.status(), 201);
    }

    // Verify search works
    let resp = client
        .post(url(port, "/indexes/recipes/search"))
        .json(&serde_json::json!({"q": "pancakes"}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().unwrap();
    let hits = body["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["document_id"], "r1");

    stop_server(child);

    // ---- SECOND SESSION (restart) ----
    let (child2, port2) = start_server(data_dir);

    // List indexes
    let client2 = reqwest::blocking::Client::new();
    let resp = client2.get(url(port2, "/indexes")).send().unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = client2
        .get(url(port2, "/indexes/recipes"))
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(body["name"], "recipes");
    assert_eq!(body["document_count"], 3);

    // Search after restart
    let resp = client2
        .post(url(port2, "/indexes/recipes/search"))
        .json(&serde_json::json!({"q": "soup"}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().unwrap();
    assert_eq!(body["hits"].as_array().unwrap().len(), 1);

    // Get individual document
    let resp = client2
        .get(url(port2, "/indexes/recipes/documents/r2"))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().unwrap();
    assert_eq!(body["id"], "r2");
    assert_eq!(body["fields"]["title"], "Omelette");

    stop_server(child2);
}

/// Test creating an index, adding docs, deleting them, and verifying
/// state is consistent after restart.
#[test]
fn test_recovery_with_deletions() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();

    // ---- FIRST SESSION ----
    let (child, port) = start_server(data_dir);
    let client = reqwest::blocking::Client::new();

    client
        .post(url(port, "/indexes"))
        .json(&serde_json::json!({"name": "items"}))
        .send()
        .unwrap();
    client
        .post(url(port, "/indexes/items/documents"))
        .json(&serde_json::json!({"id": "a", "fields": {"val": "keep"}}))
        .send()
        .unwrap();
    client
        .post(url(port, "/indexes/items/documents"))
        .json(&serde_json::json!({"id": "b", "fields": {"val": "delete_me"}}))
        .send()
        .unwrap();

    // Delete one document
    client
        .delete(url(port, "/indexes/items/documents/b"))
        .send()
        .unwrap();

    // Delete the other and recreate
    client
        .delete(url(port, "/indexes/items/documents/a"))
        .send()
        .unwrap();
    client
        .post(url(port, "/indexes/items/documents"))
        .json(&serde_json::json!({"id": "c", "fields": {"val": "new"}}))
        .send()
        .unwrap();

    stop_server(child);

    // ---- SECOND SESSION ----
    let (child2, port2) = start_server(data_dir);
    let client2 = reqwest::blocking::Client::new();

    // Verify state
    let resp = client2.get(url(port2, "/indexes/items")).send().unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().unwrap();
    assert_eq!(body["document_count"], 1);

    // "a" was deleted, "b" was deleted, "c" was added after deletion
    // Wait — "a" was deleted then "c" was added. So remaining docs: "c" only? No,
    // let me trace through:
    // Session 1:
    //   add a, add b
    //   delete b → remaining: a
    //   delete a → remaining: (empty)
    //   add c → remaining: c
    // So document_count should be 1
    assert_eq!(body["document_count"], 1);

    let resp = client2
        .get(url(port2, "/indexes/items/documents/c"))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().unwrap();
    assert_eq!(body["fields"]["val"], "new");

    stop_server(child2);
}
