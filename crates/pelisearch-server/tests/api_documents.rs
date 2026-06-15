mod common;

use common::{start_server, stop_server, url};
use serde_json::Value;

/// Set up an index and return the port for further tests.
fn setup(port: u16) {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(url(port, "/indexes"))
        .json(&serde_json::json!({"name": "products"}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 201);
}

#[test]
fn test_document_insert_retrieve_delete() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    let (child, port) = start_server(data_dir);
    let client = reqwest::blocking::Client::new();

    setup(port);

    // ---- INSERT ----
    let resp = client
        .post(url(port, "/indexes/products/documents"))
        .json(&serde_json::json!({
            "id": "doc1",
            "fields": {"title": "Widget", "price": 9.99}
        }))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: Value = resp.json().unwrap();
    assert_eq!(body["id"], "doc1");

    // ---- RETRIEVE ----
    let resp = client
        .get(url(port, "/indexes/products/documents/doc1"))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().unwrap();
    assert_eq!(body["id"], "doc1");
    assert_eq!(body["fields"]["title"], "Widget");
    assert_eq!(body["fields"]["price"], 9.99);

    // ---- RETRIEVE (not found) ----
    let resp = client
        .get(url(port, "/indexes/products/documents/nonexistent"))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 404);

    // ---- DELETE ----
    let resp = client
        .delete(url(port, "/indexes/products/documents/doc1"))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 204);

    // ---- RETRIEVE (after delete) ----
    let resp = client
        .get(url(port, "/indexes/products/documents/doc1"))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 404);

    // ---- DELETE (not found) ----
    let resp = client
        .delete(url(port, "/indexes/products/documents/nonexistent"))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 404);

    stop_server(child);
}

#[test]
fn test_document_bulk_add() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    let (child, port) = start_server(data_dir);
    let client = reqwest::blocking::Client::new();

    setup(port);

    // ---- BULK ADD ----
    let resp = client
        .post(url(port, "/indexes/products/documents/bulk"))
        .json(&serde_json::json!({
            "documents": [
                {"id": "b1", "fields": {"name": "Alpha"}},
                {"id": "b2", "fields": {"name": "Beta"}},
                {"id": "b3", "fields": {"name": "Gamma"}},
            ]
        }))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: Value = resp.json().unwrap();
    let docs = body["documents"].as_array().unwrap();
    assert_eq!(docs.len(), 3);
    for doc in docs {
        assert_eq!(doc["status"], "created");
        assert!(doc["error"].is_null());
    }

    // ---- VERIFY individual retrieval ----
    for id in &["b1", "b2", "b3"] {
        let resp = client
            .get(url(port, &format!("/indexes/products/documents/{id}")))
            .send()
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    // ---- BULK with partial errors ----
    let resp = client
        .post(url(port, "/indexes/products/documents/bulk"))
        .json(&serde_json::json!({
            "documents": [
                {"id": "new", "fields": {"name": "New"}},
                {"id": "b1", "fields": {"name": "Duplicate"}},
                {"id": "", "fields": {"name": "Empty ID"}},
            ]
        }))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: Value = resp.json().unwrap();
    let docs = body["documents"].as_array().unwrap();
    assert_eq!(docs[0]["status"], "created");
    assert_eq!(docs[1]["status"], "error");
    assert_eq!(docs[2]["status"], "error");

    // ---- BULK with empty list ----
    let resp = client
        .post(url(port, "/indexes/products/documents/bulk"))
        .json(&serde_json::json!({"documents": []}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 400);

    stop_server(child);
}

#[test]
fn test_document_insert_duplicate() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    let (child, port) = start_server(data_dir);
    let client = reqwest::blocking::Client::new();

    setup(port);

    // First insert
    let resp = client
        .post(url(port, "/indexes/products/documents"))
        .json(&serde_json::json!({"id": "d1", "fields": {"x": 1}}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 201);

    // Duplicate
    let resp = client
        .post(url(port, "/indexes/products/documents"))
        .json(&serde_json::json!({"id": "d1", "fields": {"x": 2}}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 409);

    stop_server(child);
}
