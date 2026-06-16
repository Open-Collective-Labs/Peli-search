# Documentation Search

## Document Schema

```json
{
  "id": "doc-789",
  "fields": {
    "title": "Getting Started",
    "section": "guides",
    "content": "To install PeliSearch, run `npm install @pelisearch/client`...",
    "url": "/docs/getting-started",
    "version": "1.0",
    "order": 1
  }
}
```

## Search Strategies

### Content Search with Snippets

```typescript
const results = await client.search("docs", {
  query: { match: { content: "installation" } },
  highlight: true,
})
```

### Section Filtered

```typescript
const results = await client.search("docs", {
  query: { match: { title: "API" } },
  filters: [{ term: { section: "reference" } }],
})
```

### Versioned Search

```typescript
const results = await client.search("docs", {
  query: { match: { content: "client" } },
  filters: [{ term: { version: "1.0" } }],
  sort: [{ field: "order", order: "Asc" }],
})
```
