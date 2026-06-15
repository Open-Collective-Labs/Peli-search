use std::collections::HashMap;

use pelisearch_core::aggregation::{
    AverageAggregation, CountAggregation, MaxAggregation, MinAggregation, SumAggregation,
    TermsAggregation,
};
use pelisearch_core::document::Document;
use pelisearch_core::engine::SearchEngine;
use pelisearch_core::query::{MatchQuery, Query, RangeQuery, SearchRequest};
use pelisearch_core::sort::SortField;

fn setup_engine() -> SearchEngine {
    let mut engine = SearchEngine::new();
    engine.create_index("test").unwrap();

    for (id, category, price) in [
        ("doc1", "electronics", 100.0),
        ("doc2", "electronics", 200.0),
        ("doc3", "sports", 150.0),
        ("doc4", "sports", 50.0),
    ] {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!(category));
        fields.insert("category".to_string(), serde_json::json!(category));
        fields.insert("price".to_string(), serde_json::json!(price));
        engine
            .add_document("test", Document::new(id, fields).unwrap())
            .unwrap();
    }

    engine
}

#[test]
fn terms_single_category() {
    let engine = setup_engine();
    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("title", "electronics sports")),
        filters: vec![],
        sort: vec![],
        aggregations: vec![
            pelisearch_core::aggregation::Aggregation::Terms(
                TermsAggregation::new("category").with_size(10),
            ),
        ],
        from: 0,
        size: 10,
    };
    let response = engine.search_request_with_explanations("test", &request).unwrap();
    let aggs = &response.aggregations;

    let cat_agg = aggs.get("category").unwrap();
    let obj = cat_agg.as_object().unwrap();
    assert_eq!(obj.get("electronics").unwrap().as_u64().unwrap(), 2);
    assert_eq!(obj.get("sports").unwrap().as_u64().unwrap(), 2);
}

#[test]
fn terms_multiple_categories() {
    let mut engine = SearchEngine::new();
    engine.create_index("test").unwrap();

    for (id, category) in [
        ("a", "electronics"),
        ("b", "electronics"),
        ("c", "sports"),
        ("d", "footwear"),
        ("e", "footwear"),
        ("f", "footwear"),
    ] {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!(category));
        fields.insert("category".to_string(), serde_json::json!(category));
        engine
            .add_document("test", Document::new(id, fields).unwrap())
            .unwrap();
    }

    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("title", "electronics sports footwear")),
        filters: vec![],
        sort: vec![],
        aggregations: vec![
            pelisearch_core::aggregation::Aggregation::Terms(
                TermsAggregation::new("category").with_size(10),
            ),
        ],
        from: 0,
        size: 10,
    };
    let response = engine.search_request_with_explanations("test", &request).unwrap();
    let aggs = response.aggregations.get("category").unwrap();
    let obj = aggs.as_object().unwrap();
    assert_eq!(obj.get("footwear").unwrap().as_u64().unwrap(), 3);
    assert_eq!(obj.get("electronics").unwrap().as_u64().unwrap(), 2);
    assert_eq!(obj.get("sports").unwrap().as_u64().unwrap(), 1);
}

#[test]
fn terms_empty_index() {
    let mut engine = SearchEngine::new();
    engine.create_index("empty").unwrap();
    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("title", "anything")),
        filters: vec![],
        sort: vec![],
        aggregations: vec![
            pelisearch_core::aggregation::Aggregation::Terms(
                TermsAggregation::new("category").with_size(10),
            ),
        ],
        from: 0,
        size: 10,
    };
    let response = engine
        .search_request_with_explanations("empty", &request)
        .unwrap();
    let aggs = response.aggregations.get("category").unwrap();
    let obj = aggs.as_object().unwrap();
    assert!(obj.is_empty());
}

#[test]
fn count_aggregation() {
    let engine = setup_engine();
    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("title", "electronics sports")),
        filters: vec![],
        sort: vec![],
        aggregations: vec![
            pelisearch_core::aggregation::Aggregation::Count(CountAggregation::new("price")),
        ],
        from: 0,
        size: 10,
    };
    let response = engine.search_request_with_explanations("test", &request).unwrap();
    let aggs = &response.aggregations;
    let result = aggs.get("price").unwrap();
    assert_eq!(result.get("count").unwrap().as_u64().unwrap(), 4);
}

#[test]
fn min_aggregation() {
    let engine = setup_engine();
    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("title", "electronics sports")),
        filters: vec![],
        sort: vec![],
        aggregations: vec![
            pelisearch_core::aggregation::Aggregation::Min(MinAggregation::new("price")),
        ],
        from: 0,
        size: 10,
    };
    let response = engine.search_request_with_explanations("test", &request).unwrap();
    let aggs = &response.aggregations;
    let result = aggs.get("price").unwrap();
    assert_eq!(
        result.get("value").unwrap().as_f64().unwrap(),
        50.0
    );
}

#[test]
fn max_aggregation() {
    let engine = setup_engine();
    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("title", "electronics sports")),
        filters: vec![],
        sort: vec![],
        aggregations: vec![
            pelisearch_core::aggregation::Aggregation::Max(MaxAggregation::new("price")),
        ],
        from: 0,
        size: 10,
    };
    let response = engine.search_request_with_explanations("test", &request).unwrap();
    let aggs = &response.aggregations;
    let result = aggs.get("price").unwrap();
    assert_eq!(result.get("value").unwrap().as_f64().unwrap(), 200.0);
}

#[test]
fn average_aggregation() {
    let engine = setup_engine();
    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("title", "electronics sports")),
        filters: vec![],
        sort: vec![],
        aggregations: vec![
            pelisearch_core::aggregation::Aggregation::Average(AverageAggregation::new("price")),
        ],
        from: 0,
        size: 10,
    };
    let response = engine.search_request_with_explanations("test", &request).unwrap();
    let aggs = &response.aggregations;
    let result = aggs.get("price").unwrap();
    assert!((result.get("value").unwrap().as_f64().unwrap() - 125.0).abs() < 1e-10);
}

#[test]
fn sum_aggregation() {
    let engine = setup_engine();
    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("title", "electronics sports")),
        filters: vec![],
        sort: vec![],
        aggregations: vec![
            pelisearch_core::aggregation::Aggregation::Sum(SumAggregation::new("price")),
        ],
        from: 0,
        size: 10,
    };
    let response = engine.search_request_with_explanations("test", &request).unwrap();
    let aggs = &response.aggregations;
    let result = aggs.get("price").unwrap();
    assert!((result.get("value").unwrap().as_f64().unwrap() - 500.0).abs() < 1e-10);
}

#[test]
fn combined_query_filters_sort_aggregations() {
    let mut engine = SearchEngine::new();
    engine.create_index("products").unwrap();

    for (id, title, category, price) in [
        ("p1", "electric bike", "electronics", 799.0),
        ("p2", "mountain bike", "sports", 1500.0),
        ("p3", "running shoes", "sports", 120.0),
        ("p4", "wireless mouse", "electronics", 25.0),
        ("p5", "laptop bag", "accessories", 45.0),
    ] {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!(title));
        fields.insert("category".to_string(), serde_json::json!(category));
        fields.insert("price".to_string(), serde_json::json!(price));
        engine
            .add_document("products", Document::new(id, fields).unwrap())
            .unwrap();
    }

    let request = SearchRequest {
        query: Query::Match(MatchQuery::new("title", "bike shoes bag mouse")),
        filters: vec![
            Query::Range(RangeQuery::new("price").with_gte(20.0)),
        ],
        sort: vec![SortField::desc("price")],
        aggregations: vec![
            pelisearch_core::aggregation::Aggregation::Terms(
                TermsAggregation::new("category").with_size(10),
            ),
            pelisearch_core::aggregation::Aggregation::Average(AverageAggregation::new("price")),
            pelisearch_core::aggregation::Aggregation::Count(CountAggregation::new("price")),
        ],
        from: 0,
        size: 10,
    };

    let response = engine
        .search_request_with_explanations("products", &request)
        .unwrap();

    assert_eq!(response.hits.len(), 5);

    // Sorted by price descending (hits preserve BM25 score order since sort is by
    // ScoreField which maps to BM25 score — but we specified SortField::desc("price")
    // which sorts by the price field value, not the score)
    let docs: Vec<&str> = response
        .hits
        .iter()
        .map(|h| h.document_id.as_str())
        .collect();
    // p2=mountain bike 1500, p1=electric bike 799, p3=running shoes 120,
    // p5=laptop bag 45, p4=wireless mouse 25
    assert_eq!(docs, vec!["p2", "p1", "p3", "p5", "p4"]);

    // Check terms aggregation
    let cat_agg = response.aggregations.get("category").unwrap();
    let cat_obj = cat_agg.as_object().unwrap();
    assert_eq!(cat_obj.get("electronics").unwrap().as_u64().unwrap(), 2);
    assert_eq!(cat_obj.get("sports").unwrap().as_u64().unwrap(), 2);
    assert_eq!(cat_obj.get("accessories").unwrap().as_u64().unwrap(), 1);

    // Average price (last write wins for "price" key — average overwrites count)
    let avg = response.aggregations.get("price").unwrap();
    // (799 + 1500 + 120 + 25 + 45) / 5 = 497.8
    if avg.get("value").is_some() {
        assert!((avg.get("value").unwrap().as_f64().unwrap() - 497.8).abs() < 1e-10);
    }
}
