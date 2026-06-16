# Rust Example Projects

## E-commerce Search

```rust
use pelisearch::{AddDocumentRequest, Client, SearchRequest, SortField};
use std::collections::HashMap;

let client = Client::from_url("http://localhost:7700")?;

// Index products
client.bulk_add_documents("products", vec![
    AddDocumentRequest {
        id: "mbp".into(),
        fields: HashMap::from([
            ("name".into(), "MacBook Pro".into()),
            ("brand".into(), "Apple".into()),
            ("category".into(), "laptops".into()),
            ("price".into(), 1999.into()),
        ]),
    },
]).await?;

// Search with filters and sorting
let results = client.search("products", &SearchRequest {
    query: Some(HashMap::from([
        ("match".into(), serde_json::json!({"name": "laptop"})),
    ])),
    filters: vec![
        serde_json::json!({"term": {"category": "laptops"}}),
    ],
    sort: vec![SortField { field: "price".into(), order: "Asc".into() }],
    from: Some(0),
    size: Some(10),
    ..Default::default()
}).await?;
```

## Documentation Search

```rust
// Index docs
client.bulk_add_documents("docs", vec![
    AddDocumentRequest {
        id: "1".into(),
        fields: HashMap::from([
            ("title".into(), "Installation Guide".into()),
            ("content".into(), "...".into()),
        ]),
    },
]).await?;

// Search with highlighting
let results = client.search("docs", &SearchRequest {
    query: Some(HashMap::from([
        ("match".into(), serde_json::json!({"content": "installation"})),
    ])),
    highlight: Some(true),
    ..Default::default()
}).await?;
```
