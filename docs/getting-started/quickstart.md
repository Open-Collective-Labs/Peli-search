# Quickstart

This guide walks through your first index creation, document insertion, and search in under five minutes.

## 1. Start the Server

```bash
peli serve
```

The server listens on `http://127.0.0.1:8080`.

## 2. Create an Index

```bash
curl -X POST http://127.0.0.1:8080/indexes \
  -H "Content-Type: application/json" \
  -d '{
    "name": "movies",
    "mappings": {
      "properties": {
        "title": { "type": "text" },
        "year": { "type": "integer" },
        "genre": { "type": "keyword" }
      }
    }
  }'
```

## 3. Add Documents

```bash
curl -X POST http://127.0.0.1:8080/indexes/movies/documents \
  -H "Content-Type: application/json" \
  -d '{
    "documents": [
      { "id": "1", "title": "The Matrix", "year": 1999, "genre": "sci-fi" },
      { "id": "2", "title": "Inception", "year": 2010, "genre": "sci-fi" },
      { "id": "3", "title": "The Godfather", "year": 1972, "genre": "drama" }
    ]
  }'
```

## 4. Search Documents

```bash
curl -X POST http://127.0.0.1:8080/indexes/movies/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "match": { "title": "matrix" }
    }
  }'
```

## 5. View Results

```json
{
  "hits": [
    {
      "id": "1",
      "score": 0.693,
      "document": {
        "title": "The Matrix",
        "year": 1999,
        "genre": "sci-fi"
      }
    }
  ],
  "total_hits": 1,
  "query_time_ms": 2
}
```

## Next Steps

- [First Index](./first-index.md) — learn what indexes are and how to configure them
- [First Search](./first-search.md) — explore filters, sorting, and pagination
- [Concepts](../concepts/architecture.md) — understand how the engine works
