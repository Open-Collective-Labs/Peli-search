# Searching

## Simple Full-Text Search

Search queries all fields:

### JavaScript

```typescript
const results = await client.search("articles", { q: "rust" })
```

### Python

```python
from pelisearch import SearchRequest

results = client.search("articles", SearchRequest(q="rust"))
```

### Go

```go
q := "rust"
results, err := client.Search(ctx, "articles", &pelisearch.SearchRequest{Q: &q})
```

### Rust

```rust
let results = client.search("articles", &SearchRequest {
    q: Some("rust".into()),
    ..Default::default()
}).await?;
```

## Field-Specific Match

Use `match` for BM25 full-text scoring on a specific field:

```typescript
results = await client.search("articles", {
  query: { match: { title: "rust" } }
})
```

```python
from pelisearch import MatchQuery

results = client.search("articles", SearchRequest(
    query=MatchQuery(match={"title": "rust"}),
))
```

```go
results, _ = client.Search(ctx, "articles", &pelisearch.SearchRequest{
    Query: map[string]interface{}{
        "match": map[string]string{"title": "rust"},
    },
})
```

```rust
let results = client.search("articles", &SearchRequest {
    query: Some(HashMap::from([
        ("match".into(), serde_json::json!({"title": "rust"})),
    ])),
    ..Default::default()
}).await?;
```

## Exact Term Match

Use `term` for exact, non-analyzed field matching:

```typescript
results = await client.search("products", {
  query: { term: { category: "electronics" } }
})
```

## Range Filter

Use `range` for numeric field filtering:

```typescript
results = await client.search("products", {
  query: { range: { price: { gte: 10, lte: 100 } } },
  filters: [{ term: { category: "electronics" } }]
})
```

## Sorting

Sort uses structured `SortField` objects (not strings):

### JavaScript

```typescript
results = await client.search("products", {
  query: { match: { title: "shoes" } },
  sort: [{ field: "price", order: "Asc" }]
})
```

### Python

```python
from pelisearch import SortField

results = client.search("products", SearchRequest(
    query=MatchQuery(match={"title": "shoes"}),
    sort=[SortField(field="price", order="Asc")],
))
```

### Go

```go
results, _ = client.Search(ctx, "products", &pelisearch.SearchRequest{
    Query: map[string]interface{}{
        "match": map[string]string{"title": "shoes"},
    },
    Sort: []pelisearch.SortField{
        {Field: "price", Order: "Asc"},
    },
})
```

### Rust

```rust
results = client.search("products", &SearchRequest {
    query: Some(HashMap::from([
        ("match".into(), serde_json::json!({"title": "shoes"})),
    ])),
    sort: vec![SortField { field: "price".into(), order: "Asc".into() }],
    ..Default::default()
}).await?;
```

## Pagination

Use `from` (offset) and `size` (page size):

```typescript
results = await client.search("articles", {
  query: { match: { title: "rust" } },
  from: 20,
  size: 10
})
```

```python
results = client.search("articles", SearchRequest(
    query=MatchQuery(match={"title": "rust"}),
    from_=20,
    size=10,
))
```

```go
from := 20
size := 10
results, _ = client.Search(ctx, "articles", &pelisearch.SearchRequest{
    Query: map[string]interface{}{
        "match": map[string]string{"title": "rust"},
    },
    From: &from,
    Size: &size,
})
```

```rust
results = client.search("articles", &SearchRequest {
    query: Some(HashMap::from([
        ("match".into(), serde_json::json!({"title": "rust"})),
    ])),
    from: Some(20),
    size: Some(10),
    ..Default::default()
}).await?;
```

## Highlighting

Enable term highlighting with the `highlight` flag:

```typescript
results = await client.search("articles", {
  query: { match: { content: "rust" } },
  highlight: true
})
// results.hits[0].highlighted -> { "content": "Rust is a <em>rust</em> systems language" }
```

## Response Structure

```typescript
interface SearchResponse {
  hits: SearchHit[]
  total: number
  aggregations: Record<string, unknown>
}

interface SearchHit {
  index: string
  document_id: string
  score: number
  highlighted?: Record<string, string>  // present when highlight=true
}
```

## Agent Guidance

- Prefer `query: { match: { field: "value" } }` for user-facing search
- Use `q: "value"` for simple search across all fields
- Always paginate — never request unbounded result sets
- Use `SortField` objects, not sort strings
- Enable `highlight: true` when users need to see why a result matches
- `filters` is an array of query clauses (match/term/range)
