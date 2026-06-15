mod common;

use common::{start_server, stop_server, url};
use serde_json::Value;

/// Create, read, list, and delete indexes.
#[test]
fn test_index_crud() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    let (child, port) = start_server(data_dir);
    let client = reqwest::blocking::Client::new();

    // ---- LIST (empty) ----
    let resp = client.get(url(port, "/indexes")).send().unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().unwrap();
    assert_eq!(body["indexes"].as_array().unwrap().len(), 0);

    // ---- CREATE ----
    let resp = client
        .post(url(port, "/indexes"))
        .json(&serde_json::json!({"name": "test_index"}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: Value = resp.json().unwrap();
    assert_eq!(body["name"], "test_index");

    // ---- LIST (after create) ----
    let resp = client.get(url(port, "/indexes")).send().unwrap();
    let body: Value = resp.json().unwrap();
    assert_eq!(body["indexes"].as_array().unwrap().len(), 1);
    assert_eq!(body["indexes"][0], "test_index");

    // ---- GET (by name) ----
    let resp = client.get(url(port, "/indexes/test_index")).send().unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().unwrap();
    assert_eq!(body["name"], "test_index");
    assert_eq!(body["document_count"], 0);

    // ---- GET (not found) ----
    let resp = client
        .get(url(port, "/indexes/nonexistent"))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 404);

    // ---- CREATE (duplicate) ----
    let resp = client
        .post(url(port, "/indexes"))
        .json(&serde_json::json!({"name": "test_index"}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 409);

    // ---- CREATE (empty name) ----
    let resp = client
        .post(url(port, "/indexes"))
        .json(&serde_json::json!({"name": ""}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 400);

    // ---- DELETE ----
    let resp = client
        .delete(url(port, "/indexes/test_index"))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 204);

    // ---- LIST (after delete) ----
    let resp = client.get(url(port, "/indexes")).send().unwrap();
    let body: Value = resp.json().unwrap();
    assert_eq!(body["indexes"].as_array().unwrap().len(), 0);

    // ---- DELETE (not found) ----
    let resp = client
        .delete(url(port, "/indexes/nonexistent"))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 404);

    stop_server(child);
}
