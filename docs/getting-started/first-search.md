# Your First Search

Once documents are indexed, you can search them using various query types.

## Match Queries

The most basic search — finds documents whose text fields match the query terms.

```bash
curl -X POST http://127.0.0.1:7700/indexes/movies/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "match": { "title": "godfather" }
    }
  }'
```

### How it Works

- The query text is tokenized with the same analyzer used during indexing
- Tokens are looked up in the inverted index
- Results are scored using BM25

## Filters

Narrow results without affecting relevance scoring. Filters are specified as an array.

```bash
curl -X POST http://127.0.0.1:7700/indexes/movies/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "match": { "title": "the" }
    },
    "filters": [
      { "term": { "genre": "sci-fi" } }
    ]
  }'
```

### Filter Types

| Filter | Example | Description |
|--------|---------|-------------|
| Term | `{"term": {"genre": "sci-fi"}}` | Exact match |
| Range | `{"range": {"price": {"gte": 10, "lte": 100}}}` | Numeric range |

Multiple filters are combined with AND logic.

## Sorting

```bash
curl -X POST http://127.0.0.1:7700/indexes/movies/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "title": "the" } },
    "sort": [
      { "field": "year", "order": "Desc" }
    ]
  }'
```

## Pagination

### Offset Pagination

```bash
curl -X POST http://127.0.0.1:7700/indexes/movies/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "title": "the" } },
    "from": 10,
    "size": 10
  }'
```
