# JavaScript Example Projects

## E-commerce Search

```typescript
// Index products
await client.bulkAddDocuments("products", [
  { id: "mbp", fields: { name: "MacBook Pro", brand: "Apple", category: "laptops", price: 1999 } },
  { id: "xps", fields: { name: "Dell XPS 13", brand: "Dell", category: "laptops", price: 1499 } },
])

// Search with category filter and price sort
const results = await client.search("products", {
  query: { match: { name: "laptop" } },
  filters: [{ term: { category: "laptops" } }],
  sort: [{ field: "price", order: "Asc" }],
  from: 0,
  size: 10,
})
```

## Documentation Search

```typescript
// Index documentation pages
await client.bulkAddDocuments("docs", [
  { id: "1", fields: { title: "Installation Guide", content: "...", section: "getting-started" } },
  { id: "2", fields: { title: "API Reference", content: "...", section: "reference" } },
])

// Search with highlight for snippet display
const results = await client.search("docs", {
  query: { match: { content: "installation" } },
  highlight: true,
})
```
