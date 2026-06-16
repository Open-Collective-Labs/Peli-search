# E-commerce Search

## Schema

```json
{
  "id": "unique-product-id",
  "fields": {
    "name": "MacBook Pro 14",
    "description": "Apple M3 chip, 18GB RAM, 512GB SSD",
    "brand": "Apple",
    "category": "Laptops",
    "price": 1999.99,
    "in_stock": true,
    "tags": ["apple", "laptop", "m3"]
  }
}
```

## Indexing

### Bulk Insert

```typescript
await client.bulkAddDocuments("products", [
  {
    id: "mbp-14",
    fields: {
      name: "MacBook Pro 14",
      description: "Apple M3 chip, 18GB RAM",
      brand: "Apple",
      category: "Laptops",
      price: 1999.99,
      in_stock: true,
    },
  },
  {
    id: "xps-13",
    fields: {
      name: "Dell XPS 13",
      description: "Intel i7, 16GB RAM",
      brand: "Dell",
      category: "Laptops",
      price: 1499.99,
      in_stock: true,
    },
  },
])
```

## Search Strategies

### Full-text Search

```typescript
const results = await client.search("products", {
  query: { match: { name: "laptop" } },
  sort: [{ field: "price", order: "Asc" }],
  from: 0,
  size: 20,
})
```

### Category Filtering

```typescript
const results = await client.search("products", {
  query: { match: { description: "gaming" } },
  filters: [{ term: { category: "Laptops" } }],
})
```

### Price Range

```typescript
const results = await client.search("products", {
  query: { match: { name: "monitor" } },
  filters: [{ range: { price: { gte: 200, lte: 800 } } }],
  sort: [{ field: "price", order: "Asc" }],
})
```
