use std::collections::HashMap;

use pelisearch_core::document::Document;
use pelisearch_core::engine::SearchEngine;
use pelisearch_core::query::{MatchQuery, Query, SearchRequest};
use pelisearch_core::sort::SortField;

fn setup_engine() -> SearchEngine {
    let mut engine = SearchEngine::new();
    engine.create_index("test").unwrap();

    for (id, title, price) in [
        ("doc_a", "alpha", 100.0),
        ("doc_b", "beta", 50.0),
        ("doc_c", "charlie", 75.0),
        ("doc_d", "delta", 200.0),
    ] {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!(title));
        fields.insert("price".to_string(), serde_json::json!(price));
        engine
            .add_document("test", Document::new(id, fields).unwrap())
            .unwrap();
    }

    engine
}

fn search_with_sort(engine: &SearchEngine, sort: Vec<SortField>) -> Vec<String> {
    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("title", "alpha beta charlie delta")),
        filters: vec![],
        sort,
        aggregations: vec![],
    };
    let results = engine.search_request("test", &request).unwrap();
    results.into_iter().map(|h| h.document_id).collect()
}

#[test]
fn ascending_numeric() {
    let engine = setup_engine();
    let ids = search_with_sort(&engine, vec![SortField::asc("price")]);
    assert_eq!(ids, vec!["doc_b", "doc_c", "doc_a", "doc_d"]);
}

#[test]
fn descending_numeric() {
    let engine = setup_engine();
    let ids = search_with_sort(&engine, vec![SortField::desc("price")]);
    assert_eq!(ids, vec!["doc_d", "doc_a", "doc_c", "doc_b"]);
}

#[test]
fn multi_field_sorting() {
    let mut engine = SearchEngine::new();
    engine.create_index("test").unwrap();

    for (id, category, price) in [
        ("a", "electronics", 100.0),
        ("b", "electronics", 50.0),
        ("c", "sports", 200.0),
        ("d", "sports", 50.0),
    ] {
        let mut fields = HashMap::new();
        fields.insert("category".to_string(), serde_json::json!(category));
        fields.insert("price".to_string(), serde_json::json!(price));
        engine
            .add_document("test", Document::new(id, fields).unwrap())
            .unwrap();
    }

    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("category", "electronics sports")),
        filters: vec![],
        sort: vec![SortField::asc("category"), SortField::desc("price")],
        aggregations: vec![],
    };
    let results = engine.search_request("test", &request).unwrap();
    let ids: Vec<&str> = results.iter().map(|h| h.document_id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b", "c", "d"]);
}

#[test]
fn missing_field_sorting() {
    let mut engine = SearchEngine::new();
    engine.create_index("test").unwrap();

    let mut fields = HashMap::new();
    fields.insert("title".to_string(), serde_json::json!("has price"));
    fields.insert("price".to_string(), serde_json::json!(100));
    engine
        .add_document("test", Document::new("with_price", fields).unwrap())
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("title".to_string(), serde_json::json!("no price"));
    engine
        .add_document("test", Document::new("without_price", fields).unwrap())
        .unwrap();

    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("title", "has no")),
        filters: vec![],
        sort: vec![SortField::asc("price")],
        aggregations: vec![],
    };
    let results = engine.search_request("test", &request).unwrap();
    let ids: Vec<&str> = results.iter().map(|h| h.document_id.as_str()).collect();
    assert_eq!(ids, vec!["with_price", "without_price"]);
}

#[test]
fn preserves_bm25_without_sort() {
    let engine = setup_engine();
    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("title", "alpha beta charlie delta")),
        filters: vec![],
        sort: vec![],
        aggregations: vec![],
    };
    let results = engine.search_request("test", &request).unwrap();
    assert_eq!(results.len(), 4);
    assert!(results.windows(2).all(|w| w[0].score >= w[1].score));
}
