# Sorting

## Ascending / Descending

By default, search results are sorted by relevance score descending. To sort by a field:

```bash
curl -X POST http://127.0.0.1:7700/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "sort": [
      { "field": "price", "order": "Asc" }
    ]
  }'
```

## Multi-Field Sorting

Sort by multiple fields — results are ordered by the first field, ties are broken by subsequent fields:

```bash
curl -X POST http://127.0.0.1:7700/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "sort": [
      { "field": "category", "order": "Asc" },
      { "field": "price", "order": "Desc" }
    ]
  }'
```

| Order | Behavior |
|-------|----------|
| `Asc` | Sort ascending (A → Z, low → high) |
| `Desc` | Sort descending (Z → A, high → low) |

The default sort order is `Asc`.
