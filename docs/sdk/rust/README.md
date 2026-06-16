# Rust SDK

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
pelisearch = { path = "../sdk/rust" }  # or from crates.io when published
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The Rust SDK is a workspace member of the PeliSearch repository.

## Quick Start

```rust
use pelisearch::{Client, SearchRequest};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> pelisearch::Result<()> {
    let client = Client::from_url("http://localhost:7700")?;

    client.create_index("products").await?;
    client
        .add_document(
            "products",
            "p1",
            HashMap::from([
                ("title".into(), "Widget".into()),
                ("price".into(), 9.99.into()),
            ]),
        )
        .await?;

    let results = client
        .search("products", &SearchRequest {
            q: Some("widget".into()),
            ..Default::default()
        })
        .await?;

    println!("hits: {}", results.hits.len());
    Ok(())
}
```

## Searching

```rust
// Simple query
client.search("products", &SearchRequest {
    q: Some("wireless mouse".into()),
    ..Default::default()
}).await?;

// DSL match — use serde_json for query clauses
use std::collections::HashMap;
let mut match_clause = HashMap::new();
match_clause.insert("match".to_string(), serde_json::json!({"title": "keyboard"}));

client.search("products", &SearchRequest {
    query: Some(match_clause),
    ..Default::default()
}).await?;
```

## Filtering

```rust
client.search("products", &SearchRequest {
    q: Some("keyboard".into()),
    filters: vec![serde_json::json!({"match": {"category": "electronics"}})],
    ..Default::default()
}).await?;
```

## Aggregations

```rust
let results = client.search("jobs", &SearchRequest {
    q: Some("engineer".into()),
    ..Default::default()
}).await?;

println!("{:?}", results.aggregations);
```

## Error Handling

Errors are typed as `peliSearchError`:

```rust
use pelisearch::PeliSearchError;

match client.get_index("missing").await {
    Err(PeliSearchError::Http { status, message }) => {
        eprintln!("HTTP {status}: {message}");
    }
    Err(e) => eprintln!("{e}"),
    Ok(info) => println!("{}", info.name),
}
```

Implement `Default` for `SearchRequest` to use struct update syntax with optional fields.
