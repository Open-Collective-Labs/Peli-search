# Rust SDK

## Install

```toml
[dependencies]
pelisearch = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Client

```rust
use pelisearch::Client;

let client = Client::from_url("http://localhost:7700")
    .expect("valid URL");
```

## Indexing

```rust
use pelisearch::{AddDocumentRequest, Client};
use std::collections::HashMap;

// Create index
client.create_index("products").await?;

// Add document
let fields = HashMap::from([
    ("name".into(), "Wireless Mouse".into()),
    ("category".into(), "electronics".into()),
    ("price".into(), 29.99.into()),
]);
client.add_document("products", "p1", fields).await?;

// Bulk add
client.bulk_add_documents("products", vec![
    AddDocumentRequest {
        id: "p2".into(),
        fields: HashMap::from([
            ("name".into(), "Keyboard".into()),
        ]),
    },
]).await?;

// List indexes
let indexes: Vec<String> = client.list_indexes().await?;

// Get index info
let info = client.get_index("products").await?;

// Delete document / index
client.delete_document("products", "p1").await?;
client.delete_index("products").await?;
```

## Searching

```rust
use pelisearch::{Client, SearchRequest, SortField};
use std::collections::HashMap;

// Simple text search
let results = client.search("products", &SearchRequest {
    q: Some("mouse".into()),
    ..Default::default()
}).await?;
for hit in &results.hits {
    println!("{}: {}", hit.document_id, hit.score);
}

// Field-specific match
let results = client.search("products", &SearchRequest {
    query: Some(HashMap::from([
        ("match".into(), serde_json::json!({"name": "keyboard"})),
    ])),
    ..Default::default()
}).await?;

// With filters, sort, pagination, highlighting
let results = client.search("products", &SearchRequest {
    query: Some(HashMap::from([
        ("match".into(), serde_json::json!({"name": "keyboard"})),
    ])),
    filters: vec![
        serde_json::json!({"term": {"category": "electronics"}}),
    ],
    sort: vec![
        SortField { field: "price".into(), order: "Asc".into() },
    ],
    from: Some(0),
    size: Some(20),
    highlight: Some(true),
    ..Default::default()
}).await?;
```

## Response

```rust
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub aggregations: HashMap<String, serde_json::Value>,
    pub total: usize,
}

pub struct SearchHit {
    pub index: String,
    pub document_id: String,
    pub score: f64,
    pub highlighted: Option<HashMap<String, String>>,
}
```

## Error Handling

```rust
match client.get_index("missing").await {
    Err(pelisearch::PeliSearchError::Http { status, message }) => {
        eprintln!("HTTP {status}: {message}");
    }
    Err(e) => eprintln!("{e}"),
    Ok(info) => println!("{}", info.name),
}
```
