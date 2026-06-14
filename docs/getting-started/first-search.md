# Your First Search

Once documents are indexed, you can search them using various query types.

## Match Queries

The most basic search — finds documents whose text fields match the query terms.

```bash
curl -X POST http://127.0.0.1:8080/indexes/movies/search \
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

Narrow results without affecting relevance scoring.

```bash
curl -X POST http://127.0.0.1:8080/indexes/movies/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "match": { "title": "the" }
    },
    "filter": {
      "term": { "genre": "sci-fi" }
    }
  }'
```

### Filter Types

| Filter | Example | Description |
|--------|---------|-------------|
| Term | `{"term": {"genre": "sci-fi"}}` | Exact match |
| Range | `{"range": {"year": {"gte": 2000}}}` | Numeric/date range |
| Exists | `{"exists": {"field": "rating"}}` | Field exists |

## Sorting

```bash
curl -X POST http://127.0.0.1:8080/indexes/movies/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "title": "the" } },
    "sort": [
      { "year": "desc" },
      "_score"
    ]
  }'
```

## Pagination

### Offset Pagination

```bash
curl -X POST http://127.0.0.1:8080/indexes/movies/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "title": "the" } },
    "from": 10,
    "size": 10
  }'
```

### Cursor Pagination (recommended for deep pages)

```bash
curl -X POST http://127.0.0.1:8080/indexes/movies/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "title": "the" } },
    "size": 10,
    "cursor": "eyJpZCI6IjEwIn0="
  }'
```
