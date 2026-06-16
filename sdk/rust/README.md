# PeliSearch Rust SDK

Official Rust client for [PeliSearch](https://github.com/Open-Collective-Labs/Peli-search).

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
pelisearch = "0.1.0"
```

Requires Rust edition 2024+.

## Quick Start

```rust
use pelisearch::{Client, SearchRequest};

let client = Client::from_url("http://localhost:7700")?;

client.create_index("products").await?;
client.add_document("products", "doc1", map! {
    "title".into(): "Widget".into(),
    "price".into(): 9.99.into(),
}).await?;

let results = client.search("products", &SearchRequest {
    q: Some("widget".into()),
    ..Default::default()
}).await?;

for hit in results.hits {
    println!("{}: {}", hit.document_id, hit.score);
}
```

## Documentation

See the [full SDK docs](https://github.com/Open-Collective-Labs/Peli-search) for detailed usage.

## License

MIT
