# Quickstart

This guide walks through your first index creation, document insertion, and search in under five minutes.

## 1. Start the Server

```bash
pelisearch-server
```

The server listens on `http://127.0.0.1:7700`.

## 2. Create an Index

```bash
curl -X POST http://127.0.0.1:7700/indexes \
  -H "Content-Type: application/json" \
  -d '{
    "name": "movies"
  }'
```

## 3. Add Documents

```bash
curl -X POST http://127.0.0.1:7700/indexes/movies/documents \
  -H "Content-Type: application/json" \
  -d '{
    "id": "1",
    "fields": {
      "title": "The Matrix",
      "year": 1999,
      "genre": "sci-fi"
    }
  }'
```

## 4. Search Documents

```bash
curl -X POST http://127.0.0.1:7700/indexes/movies/search \
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
      "index": "movies",
      "document_id": "1",
      "score": 0.693
    }
  ],
  "total": 1,
  "aggregations": {}
}
```

## Next Steps

- [First Index](./first-index.md) — learn what indexes are and how to configure them
- [First Search](./first-search.md) — explore filters, sorting, and pagination
- [Concepts](../concepts/architecture.md) — understand how the engine works
