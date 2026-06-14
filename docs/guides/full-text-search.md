# Full-Text Search

## Match Query

The match query is the standard way to perform full-text search:

```bash
curl -X POST http://127.0.0.1:8080/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "match": {
        "title": "quick brown fox"
      }
    }
  }'
```

The query text is analyzed (tokenized, lowercased, stemmed) before lookup. Documents matching any of the resulting tokens are returned, scored by BM25.

### Match Options

| Option | Default | Description |
|--------|---------|-------------|
| `operator` | `or` | `or` or `and` — whether all terms must match |
| `minimum_should_match` | — | Minimum number of terms that must match |
| `fuzziness` | `0` | Levenshtein edit distance for fuzzy matching |

```bash
curl -X POST http://127.0.0.1:8080/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "match": {
        "title": {
          "query": "quick brown fox",
          "operator": "and",
          "fuzziness": 1
        }
      }
    }
  }'
```

## Phrase Search

Find documents where terms appear in the exact order:

```bash
curl -X POST http://127.0.0.1:8080/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "phrase": {
        "body": "to be or not to be"
      }
    }
  }'
```

### Slop

Allow terms to be reordered within a distance:

```bash
curl -X POST http://127.0.0.1:8080/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "phrase": {
        "body": {
          "query": "quick fox",
          "slop": 2
        }
      }
    }
  }'
```

## Boolean Queries

Combine multiple query clauses:

```bash
curl -X POST http://127.0.0.1:8080/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "boolean": {
        "must": [
          { "match": { "title": "search" } }
        ],
        "filter": [
          { "term": { "status": "published" } }
        ],
        "must_not": [
          { "match": { "category": "spam" } }
        ],
        "should": [
          { "match": { "tags": "tutorial" } }
        ]
      }
    }
  }'
```

| Clause | Behavior |
|--------|----------|
| `must` | Required — contributes to score |
| `filter` | Required — does not affect score |
| `must_not` | Excluded |
| `should` | Optional — increases score of matching docs |
