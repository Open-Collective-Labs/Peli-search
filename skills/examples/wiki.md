# Wiki / Knowledge Base Search

## Document Schema

```json
{
  "id": "article-123",
  "fields": {
    "title": "Rust (programming language)",
    "summary": "Rust is a multi-paradigm, high-level, general-purpose programming language.",
    "content": "Rust emphasizes performance, type safety, and concurrency...",
    "category": "Programming",
    "tags": ["systems", "programming", "rust"],
    "last_updated": "2025-06-01"
  }
}
```

## Search Strategies

### Title-Boosted Search

```typescript
const results = await client.search("wiki", {
  query: { match: { title: "rust" } },
})
```

### Full Content Search with Highlighting

```typescript
const results = await client.search("wiki", {
  query: { match: { content: "memory safety" } },
  highlight: true,
})
```

### Category Filtered

```typescript
const results = await client.search("wiki", {
  query: { match: { title: "concurrency" } },
  filters: [{ term: { category: "Programming" } }],
})
```
