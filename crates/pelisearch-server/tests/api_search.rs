mod common;

use common::{start_server, stop_server, url};
use serde_json::Value;

/// Set up an index with some documents.
fn setup(port: u16) {
    let client = reqwest::blocking::Client::new();

    // Create index
    client
        .post(url(port, "/indexes"))
        .json(&serde_json::json!({"name": "store"}))
        .send()
        .unwrap();

    // Add documents
    let docs = vec![
        ("p1", "Wireless Mouse", "electronics", 29.99),
        ("p2", "Mechanical Keyboard", "electronics", 89.99),
        ("p3", "Running Shoes", "sports", 120.00),
        ("p4", "Yoga Mat", "sports", 25.00),
        ("p5", "Novel: The Gateway", "books", 14.99),
    ];

    for (id, title, category, price) in docs {
        client
            .post(url(port, "/indexes/store/documents"))
            .json(&serde_json::json!({
                "id": id,
                "fields": {
                    "title": title,
                    "category": category,
                    "price": price,
                }
            }))
            .send()
            .unwrap();
    }
}

#[test]
fn test_search_legacy_q() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    let (child, port) = start_server(data_dir);
    let client = reqwest::blocking::Client::new();

    setup(port);

    // Legacy `q` param — search all fields
    let resp = client
        .post(url(port, "/indexes/store/search"))
        .json(&serde_json::json!({"q": "mouse"}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().unwrap();
    let hits = body["hits"].as_array().unwrap();
    assert!(!hits.is_empty(), "expected at least one hit for 'mouse'");
    assert_eq!(body["aggregations"], serde_json::json!({}));

    stop_server(child);
}

#[test]
fn test_search_dsl_match() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    let (child, port) = start_server(data_dir);
    let client = reqwest::blocking::Client::new();

    setup(port);

    // DSL match query
    let resp = client
        .post(url(port, "/indexes/store/search"))
        .json(&serde_json::json!({
            "query": {"match": {"title": "keyboard"}}
        }))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().unwrap();
    let hits = body["hits"].as_array().unwrap();
    assert!(!hits.is_empty(), "expected at least one hit for 'keyboard'");
    assert_eq!(body["aggregations"], serde_json::json!({}));

    stop_server(child);
}

#[test]
fn test_search_no_results() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    let (child, port) = start_server(data_dir);
    let client = reqwest::blocking::Client::new();

    setup(port);

    // No match
    let resp = client
        .post(url(port, "/indexes/store/search"))
        .json(&serde_json::json!({"q": "xyznonexistent"}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().unwrap();
    assert!(body["hits"].as_array().unwrap().is_empty());

    stop_server(child);
}

#[test]
fn test_search_invalid_query() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    let (child, port) = start_server(data_dir);
    let client = reqwest::blocking::Client::new();

    setup(port);

    // Empty query clause
    let resp = client
        .post(url(port, "/indexes/store/search"))
        .json(&serde_json::json!({"query": {}}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 400);

    stop_server(child);
}

#[test]
fn test_search_index_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    let (child, port) = start_server(data_dir);
    let client = reqwest::blocking::Client::new();

    let resp = client
        .post(url(port, "/indexes/nonexistent/search"))
        .json(&serde_json::json!({"q": "test"}))
        .send()
        .unwrap();
    assert_eq!(resp.status(), 404);

    stop_server(child);
}
