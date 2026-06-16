# Rust Search Examples

```rust
use pelisearch::{Client, SearchRequest, SortField};
use std::collections::HashMap;

// Simple query
let results = client.search("articles", &SearchRequest {
    q: Some("rust".into()),
    ..Default::default()
}).await?;
println!("Found {} results", results.total);

// Field match with highlighting
let results = client.search("articles", &SearchRequest {
    query: Some(HashMap::from([
        ("match".into(), serde_json::json!({"title": "rust"})),
    ])),
    highlight: Some(true),
    ..Default::default()
}).await?;
for hit in &results.hits {
    if let Some(ref hl) = hit.highlighted {
        println!("{:?}", hl.get("title"));
    }
}

// Filtered search
let results = client.search("products", &SearchRequest {
    query: Some(HashMap::from([
        ("match".into(), serde_json::json!({"name": "keyboard"})),
    ])),
    filters: vec![
        serde_json::json!({"range": {"price": {"gte": 50, "lte": 200}}}),
    ],
    ..Default::default()
}).await?;

// Paginated and sorted
let results = client.search("products", &SearchRequest {
    query: Some(HashMap::from([
        ("match".into(), serde_json::json!({"category": "electronics"})),
    ])),
    sort: vec![SortField { field: "price".into(), order: "Asc".into() }],
    from: Some(0),
    size: Some(20),
    ..Default::default()
}).await?;
```
