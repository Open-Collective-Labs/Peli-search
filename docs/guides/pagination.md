# Pagination

## Offset Pagination

Offset pagination uses `from` and `size` to navigate results:

```bash
curl -X POST http://127.0.0.1:7700/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "from": 20,
    "size": 10
  }'
```

| Parameter | Default | Description |
|-----------|---------|-------------|
| `from` | 0 | Number of results to skip |
| `size` | 10 | Number of results to return |
