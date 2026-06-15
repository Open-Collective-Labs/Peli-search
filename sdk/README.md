# PeliSearch SDKs

Official client libraries for [PeliSearch](https://github.com/Open-Collective-Labs/Peli-search).

| Language | Package | Status |
|----------|---------|--------|
| JavaScript | `@pelisearch/client` | ✅ |
| Python | `pelisearch` | ✅ |
| Go | `github.com/Open-Collective-Labs/Peli-search/sdk/go/pelisearch` | ✅ |
| Rust | `pelisearch` | ✅ |

## Usage

### JavaScript / TypeScript

```ts
import { PeliSearchClient } from "@pelisearch/client"

const client = new PeliSearchClient({ host: "http://localhost:7700" })

await client.createIndex("products")
await client.addDocument("products", "doc1", { title: "Widget", price: 9.99 })

const results = await client.search("products", { q: "widget" })
```

### Python

```python
from pelisearch import PeliSearch, SearchRequest

client = PeliSearch("http://localhost:7700")

client.create_index("products")
client.add_document("products", "doc1", {"title": "Widget", "price": 9.99})

results = client.search("products", SearchRequest(q="widget"))
```

### Go

```go
import (
    "context"
    "github.com/Open-Collective-Labs/Peli-search/sdk/go/pelisearch"
)

client := pelisearch.NewClientFromURL("http://localhost:7700")

ctx := context.Background()
client.CreateIndex(ctx, "products")
client.AddDocument(ctx, "products", "doc1", map[string]interface{}{"title": "Widget", "price": 9.99})

q := "widget"
results, _ := client.Search(ctx, "products", &pelisearch.SearchRequest{Q: &q})
```

### Rust

```rust
use pelisearch::{Client, SearchRequest};

let client = Client::from_url("http://localhost:7700")?;

client.create_index("products").await?;
client.add_document("products", "doc1", map! {"title".into(): "Widget".into(), "price".into(): 9.99.into()}).await?;

let results = client.search("products", &SearchRequest { q: Some("widget".into()), ..Default::default() }).await?;
```

## Documentation

- [SDK docs](../docs/sdk/README.md)
- [Examples](../examples/)
- [Integration tests](./tests/)

## Shared Models

See [`shared/`](./shared/) for OpenAPI spec, JSON schemas, and cross-language type reference.
