# Indexing

## Creating an Index

Indexes must be created before documents can be inserted. Use lowercase, no spaces.

### JavaScript

```typescript
await client.createIndex("articles")
```

### Python

```python
client.create_index("articles")
```

### Go

```go
ctx := context.Background()
client.CreateIndex(ctx, "articles")
```

### Rust

```rust
client.create_index("articles").await?;
```

## Adding a Single Document

Every document needs a unique `id` and a `fields` map:

### JavaScript

```typescript
await client.addDocument("articles", "doc-1", {
  title: "Rust Programming",
  content: "Rust is a systems programming language..."
})
```

### Python

```python
client.add_document("articles", "doc-1", {
    "title": "Rust Programming",
    "content": "Rust is a systems programming language..."
})
```

### Go

```go
client.AddDocument(ctx, "articles", "doc-1", map[string]interface{}{
    "title":   "Rust Programming",
    "content": "Rust is a systems programming language...",
})
```

### Rust

```rust
use std::collections::HashMap;

let fields = HashMap::from([
    ("title".into(), "Rust Programming".into()),
    ("content".into(), "Rust is a systems programming language...".into()),
]);
client.add_document("articles", "doc-1", fields).await?;
```

## Bulk Adding Documents

Always prefer bulk insert when adding multiple documents:

### JavaScript

```typescript
await client.bulkAddDocuments("articles", [
  { id: "1", fields: { title: "Rust", content: "..." } },
  { id: "2", fields: { title: "Go", content: "..." } },
])
```

### Python

```python
client.bulk_add_documents("articles", [
    {"id": "1", "fields": {"title": "Rust", "content": "..."}},
    {"id": "2", "fields": {"title": "Go", "content": "..."}},
])
```

### Go

```go
client.BulkAddDocuments(ctx, "articles", []map[string]interface{}{
    {"id": "1", "fields": map[string]interface{}{"title": "Rust", "content": "..."}},
    {"id": "2", "fields": map[string]interface{}{"title": "Go", "content": "..."}},
})
```

### Rust

```rust
use pelisearch::AddDocumentRequest;

client.bulk_add_documents("articles", vec![
    AddDocumentRequest {
        id: "1".into(),
        fields: HashMap::from([("title".into(), "Rust".into())]),
    },
]).await?;
```

## Agent Guidance

- Create the index **before** inserting documents
- Every document must have a unique `id` string
- Use `add_document` for a single doc, `bulk_add_documents` for multiple
- Always prefer batch inserts over individual ones
- Index names should be lowercase with no spaces
