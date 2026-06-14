# Filtering

Filters narrow search results without affecting relevance scoring. They are applied after the query but before sorting.

## Term Filters

Match documents where a field equals an exact value:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "filter": {
      "term": { "brand": "apple" }
    }
  }'
```

Multiple term filters:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "filter": {
      "terms": { "brand": ["apple", "dell"] }
    }
  }'
```

## Range Filters

Match documents within a numeric or date range:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "filter": {
      "range": {
        "price": {
          "gte": 500,
          "lte": 2000
        }
      }
    }
  }'
```

| Operator | Meaning |
|----------|---------|
| `gte` | Greater than or equal |
| `gt` | Greater than |
| `lte` | Less than or equal |
| `lt` | Less than |

## Exists Filters

Match documents that have a value for a specific field:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "filter": {
      "exists": { "field": "rating" }
    }
  }'
```

## Combining Filters

Multiple filters are combined with `and` logic by default:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "filter": {
      "and": [
        { "term": { "brand": "apple" } },
        { "range": { "price": { "gte": 1000 } } },
        { "exists": { "field": "in_stock" } }
      ]
    }
  }'
```
