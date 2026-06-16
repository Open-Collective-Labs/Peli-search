# Filtering

Filters narrow search results without affecting relevance scoring. They are applied after the query but before sorting.

## Term Filters

Match documents where a field equals an exact value:

```bash
curl -X POST http://127.0.0.1:7700/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "filters": [
      { "term": { "brand": "apple" } }
    ]
  }'
```

## Range Filters

Match documents within a numeric range:

```bash
curl -X POST http://127.0.0.1:7700/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "filters": [
      {
        "range": {
          "price": {
            "gte": 500,
            "lte": 2000
          }
        }
      }
    ]
  }'
```

| Operator | Meaning |
|----------|---------|
| `gte` | Greater than or equal |
| `gt` | Greater than |
| `lte` | Less than or equal |
| `lt` | Less than |
