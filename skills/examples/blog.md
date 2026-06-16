# Blog Search

## Document Schema

```json
{
  "id": "post-456",
  "fields": {
    "title": "Understanding Async/Await in Rust",
    "excerpt": "A deep dive into Rust's async model...",
    "body": "Rust's async/await syntax provides zero-cost abstractions...",
    "author": "Jane Doe",
    "tags": ["rust", "async", "programming"],
    "published_at": "2025-05-15",
    "status": "published"
  }
}
```

## Search Strategies

### Search by Title

```typescript
const results = await client.search("blog", {
  query: { match: { title: "async" } },
})
```

### Author Filter

```typescript
const results = await client.search("blog", {
  query: { match: { title: "rust" } },
  filters: [{ term: { author: "Jane Doe" } }],
})
```

### Published Status

```typescript
const results = await client.search("blog", {
  query: { match: { body: "zero-cost" } },
  filters: [{ term: { status: "published" } }],
  sort: [{ field: "published_at", order: "Desc" }],
  highlight: true,
})
```
