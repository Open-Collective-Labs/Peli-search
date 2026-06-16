# Full-Text Search

## Match Query

The match query is the standard way to perform full-text search:

```bash
curl -X POST http://127.0.0.1:7700/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "match": {
        "title": "quick brown fox"
      }
    }
  }'
```

The query text is analyzed (tokenized, lowercased) before lookup. Documents matching any of the resulting tokens are returned, scored by BM25.

## Term Query

For exact, non-analyzed field matching:

```bash
curl -X POST http://127.0.0.1:7700/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "term": {
        "category": "electronics"
      }
    }
  }'
```

## Range Query

For numeric field filtering:

```bash
curl -X POST http://127.0.0.1:7700/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "range": {
        "price": {
          "gte": 10,
          "lte": 100
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
