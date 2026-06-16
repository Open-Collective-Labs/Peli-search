# JavaScript Search Examples

## Simple Query

```typescript
const results = await client.search("articles", { q: "rust" })
console.log(`Found ${results.total} results`)
results.hits.forEach(h => console.log(h.document_id, h.score))
```

## Field Match with Highlighting

```typescript
const results = await client.search("articles", {
  query: { match: { title: "rust" } },
  highlight: true,
})
for (const hit of results.hits) {
  if (hit.highlighted) {
    console.log(hit.highlighted.title) // "<em>Rust</em> Programming"
  }
}
```

## Filtered Search

```typescript
const results = await client.search("products", {
  query: { match: { name: "keyboard" } },
  filters: [
    { range: { price: { gte: 50, lte: 200 } } },
  ],
})
```

## Paginated and Sorted

```typescript
const results = await client.search("products", {
  query: { match: { category: "electronics" } },
  sort: [{ field: "price", order: "Asc" }],
  from: 0,
  size: 20,
})
```
