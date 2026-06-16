# JavaScript SDK

## Install

```bash
npm install @pelisearch/client
```

## Client

```typescript
import { PeliSearchClient } from "@pelisearch/client"

const client = new PeliSearchClient()
// or: new PeliSearchClient({ host: "http://localhost:7700" })
```

## Indexing

```typescript
// Create index
await client.createIndex("products")

// Add document
await client.addDocument("products", "p1", {
  name: "Wireless Mouse",
  category: "electronics",
  price: 29.99
})

// Bulk add
await client.bulkAddDocuments("products", [
  { id: "p2", fields: { name: "Keyboard", category: "electronics", price: 89.99 } },
  { id: "p3", fields: { name: "Monitor", category: "electronics", price: 299.99 } },
])

// List indexes
const indexes: string[] = await client.listIndexes()

// Get index info
const info = await client.getIndex("products")

// Delete document
await client.deleteDocument("products", "p1")

// Delete index
await client.deleteIndex("products")
```

## Searching

```typescript
import type { SortField, SearchHit } from "@pelisearch/client"

// Simple text search (all fields)
const results = await client.search("products", { q: "mouse" })
for (const hit of results.hits) {
  console.log(hit.document_id, hit.score)
}

// Field-specific match
results = await client.search("products", {
  query: { match: { name: "keyboard" } },
})

// With filters, sort, pagination, and highlighting
results = await client.search("products", {
  query: { match: { name: "keyboard" } },
  filters: [{ term: { category: "electronics" } }],
  sort: [{ field: "price", order: "Asc" }],
  from: 0,
  size: 20,
  highlight: true,
})
```

## Response

```typescript
interface SearchResponse {
  hits: SearchHit[]
  total: number
  aggregations: Record<string, unknown>
}
```

## Error Handling

```typescript
import { PeliSearchError, isPeliSearchError } from "@pelisearch/client"

try {
  await client.getIndex("missing")
} catch (err) {
  if (isPeliSearchError(err)) {
    console.error(`HTTP ${err.status}: ${err.message}`)
  }
}
```
