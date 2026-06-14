# Pagination

## Offset Pagination

Offset pagination uses `from` and `size` to navigate results:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
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

### Limitations

- Offset pagination becomes expensive for deep pages (e.g., page 1000) because all results up to the offset must be scored and sorted
- Maximum `from` value is 10,000 by default

## Cursor Pagination

Cursor pagination is recommended for deep pages or real-time feeds:

```bash
# First page
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "size": 10
  }'
```

The response includes a `cursor` field:

```json
{
  "hits": [...],
  "cursor": "eyJzb3J0IjpbMTI5OV0sImlkIjoiZG9jXzQyIn0=",
  "total_hits": 2500
}
```

Pass the cursor to get the next page:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "size": 10,
    "cursor": "eyJzb3J0IjpbMTI5OV0sImlkIjoiZG9jXzQyIn0="
  }'
```

### Advantages

- O(1) per page regardless of depth
- Consistent results even if documents are added between pages
- No skipped or duplicated results
