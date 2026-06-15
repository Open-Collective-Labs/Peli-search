use std::collections::HashMap;

use pelisearch_core::document::Document;
use pelisearch_core::index::IndexManager;
use pelisearch_core::schema::{Field, FieldType, Mapping};

/// Helper to build a document with string fields from a list of key-value pairs.
fn make_doc(id: &str, fields: Vec<(&str, &str)>) -> Document {
    let mut map = HashMap::new();
    for (k, v) in fields {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    Document::new(id, map).unwrap()
}

/// Helper to build a document with arbitrary JSON values.
fn make_doc_json(id: &str, fields: Vec<(&str, serde_json::Value)>) -> Document {
    let mut map = HashMap::new();
    for (k, v) in fields {
        map.insert(k.to_string(), v);
    }
    Document::new(id, map).unwrap()
}

fn product_mapping() -> Mapping {
    Mapping::new(vec![
        Field::new("title", FieldType::Text, true),
        Field::new("category", FieldType::Keyword, false),
        Field::new("price", FieldType::Float, false),
    ])
}

fn article_mapping() -> Mapping {
    Mapping::new(vec![
        Field::new("title", FieldType::Text, true),
        Field::new("author", FieldType::Keyword, true),
        Field::new("body", FieldType::Text, true),
    ])
}

fn user_mapping() -> Mapping {
    Mapping::new(vec![
        Field::new("username", FieldType::Keyword, true),
        Field::new("bio", FieldType::Text, false),
    ])
}

#[test]
fn create_multiple_indexes() {
    let mut manager = IndexManager::new();

    manager
        .create_index_with_mapping("products", product_mapping())
        .unwrap();
    manager
        .create_index_with_mapping("articles", article_mapping())
        .unwrap();
    manager
        .create_index_with_mapping("users", user_mapping())
        .unwrap();

    assert!(manager.index_exists("products"));
    assert!(manager.index_exists("articles"));
    assert!(manager.index_exists("users"));

    let names = manager.list_indexes();
    assert_eq!(names, vec!["articles", "products", "users"]);
}

#[test]
fn search_isolation_products_vs_articles() {
    let mut manager = IndexManager::new();
    manager
        .create_index_with_mapping("products", product_mapping())
        .unwrap();
    manager
        .create_index_with_mapping("articles", article_mapping())
        .unwrap();

    let bike = make_doc("bike_1", vec![("title", "electric bike"), ("category", "outdoor")]);
    manager.add_document("products", bike).unwrap();

    let space = make_doc("art_1", vec![("title", "space exploration"), ("author", "Alice"), ("body", "deep space travel")]);
    manager.add_document("articles", space).unwrap();

    let product_results = manager.search("products", "bike").unwrap();
    assert_eq!(product_results.len(), 1);
    assert_eq!(product_results[0].document_id, "bike_1");
    assert_eq!(product_results[0].index, "products");

    let article_results = manager.search("articles", "bike").unwrap();
    assert_eq!(article_results.len(), 0);

    let article_results = manager.search("articles", "space").unwrap();
    assert_eq!(article_results.len(), 1);
    assert_eq!(article_results[0].document_id, "art_1");
    assert_eq!(article_results[0].index, "articles");

    let product_results = manager.search("products", "space").unwrap();
    assert_eq!(product_results.len(), 0);
}

#[test]
fn mapping_validation_rejects_invalid_documents() {
    let mut manager = IndexManager::new();
    manager
        .create_index_with_mapping("products", product_mapping())
        .unwrap();

    // Missing required field 'title'
    let no_title = make_doc("bad_1", vec![("category", "misc")]);
    let err = manager.add_document("products", no_title).unwrap_err();
    assert!(
        format!("{err}").contains("missing required field 'title'"),
        "expected missing-field error, got: {err}"
    );

    // Wrong type for price (string instead of float)
    let bad_price = make_doc_json(
        "bad_2",
        vec![("title", serde_json::json!("Widget")), ("price", serde_json::json!("not_a_number"))],
    );
    let err = manager.add_document("products", bad_price).unwrap_err();
    assert!(
        format!("{err}").contains("expected type"),
        "expected type-mismatch error, got: {err}"
    );

    // Valid document still works
    let valid = make_doc_json(
        "good_1",
        vec![
            ("title", serde_json::json!("Valid Product")),
            ("price", serde_json::json!(19.99)),
        ],
    );
    manager.add_document("products", valid).unwrap();
    assert_eq!(manager.search("products", "Valid").unwrap().len(), 1);
}

#[test]
fn deleted_index_unavailable() {
    let mut manager = IndexManager::new();
    manager
        .create_index_with_mapping("products", product_mapping())
        .unwrap();

    let doc = make_doc("doc1", vec![("title", "test product")]);
    manager.add_document("products", doc).unwrap();

    assert!(manager.index_exists("products"));
    assert_eq!(manager.search("products", "test").unwrap().len(), 1);

    manager.delete_index("products").unwrap();
    assert!(!manager.index_exists("products"));

    // Searching a deleted index should fail
    let err = manager.search("products", "test").unwrap_err();
    assert!(
        format!("{err}").contains("not found"),
        "expected not-found error, got: {err}"
    );

    // Adding to a deleted index should fail
    let doc2 = make_doc("doc2", vec![("title", "another")]);
    let err = manager.add_document("products", doc2).unwrap_err();
    assert!(
        format!("{err}").contains("not found"),
        "expected not-found error, got: {err}"
    );

    // Re-creating the index should work
    manager
        .create_index_with_mapping("products", product_mapping())
        .unwrap();
    assert!(manager.index_exists("products"));
}
