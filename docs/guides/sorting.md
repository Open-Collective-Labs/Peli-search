# Sorting

## Ascending / Descending

By default, search results are sorted by relevance score descending. To sort by a field:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "sort": [
      { "price": "asc" }
    ]
  }'
```

## Multi-Field Sorting

Sort by multiple fields — results are ordered by the first field, ties are broken by subsequent fields:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "sort": [
      { "category": "asc" },
      { "price": "desc" },
      "_score"
    ]
  }'
```

## Sort Modes

| Mode | Syntax | Description |
|------|--------|-------------|
| Field ascending | `{"field": "asc"}` | Sort field A → Z |
| Field descending | `{"field": "desc"}` | Sort field Z → A |
| Relevance | `"_score"` | Sort by BM25 score |
| Document ID | `"_id"` | Sort by document ID |

## Missing Values

Control how documents with missing sort fields are ordered:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "sort": [
      {
        "rating": {
          "order": "desc",
          "missing": "_last"
        }
      }
    ]
  }'
```

| Strategy | Behavior |
|----------|----------|
| `_last` (default) | Documents without the field appear last |
| `_first` | Documents without the field appear first |
