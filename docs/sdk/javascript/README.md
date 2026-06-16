# JavaScript SDK

## Installation

```bash
npm install @pelisearch/client
```

For local development from this repository:

```bash
cd sdk/javascript && npm install && npm run build
```

Requires Node.js 18+ (uses native `fetch`).

## Quick Start

```typescript
import { PeliSearchClient } from "@pelisearch/client"

const client = new PeliSearchClient({ host: "http://localhost:7700" })

await client.createIndex("products")
await client.addDocument("products", "p1", { title: "Widget", price: 9.99 })

const results = await client.search("products", { q: "widget" })
console.log(results.hits)
```

The host defaults to `http://localhost:7700` when omitted:

```typescript
const client = new PeliSearchClient()
```

## Searching

Simple full-text search:

```typescript
await client.search("products", { q: "wireless mouse" })
```

Query DSL with field-specific match:

```typescript
await client.search("products", {
  query: { match: { title: "keyboard" } },
})
```

Pagination and sorting:

```typescript
await client.search("products", {
  q: "shoes",
  from: 20,
  size: 20,
  sort: [{ field: "price", order: "Asc" }],
})
```

## Filtering

Use filter clauses to narrow results:

```typescript
await client.search("products", {
  q: "keyboard",
  filters: [{ match: { category: "electronics" } }],
})
```

## Aggregations

Aggregation results are returned in the `aggregations` field:

```typescript
const results = await client.search("products", {
  q: "engineer",
})

console.log(results.aggregations)
```

## Error Handling

HTTP errors throw `PeliSearchError` with a `status` code and optional `body`:

```typescript
import { PeliSearchClient, PeliSearchError, isPeliSearchError } from "@pelisearch/client"

try {
  await client.getIndex("missing")
} catch (err) {
  if (isPeliSearchError(err)) {
    console.error(err.status, err.message)
  }
}
```
