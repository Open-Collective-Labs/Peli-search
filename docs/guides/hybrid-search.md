# Hybrid Search

> **Phase 2 feature** — not yet available in v1.

Hybrid search combines BM25 (keyword) and vector (semantic) search to get the best of both approaches.

## Why Hybrid Search?

| Approach | Strengths | Weaknesses |
|----------|-----------|------------|
| BM25 | Exact term matching, handles rare terms | Misses synonyms, semantic relationships |
| Vector | Captures meaning, handles synonyms | May miss exact term matches |
| Hybrid | Both precision and recall | Higher computational cost |

## Combining BM25 and Vectors

```bash
curl -X POST http://127.0.0.1:8080/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "hybrid": {
        "text_query": {
          "match": { "title": "machine learning" }
        },
        "vector_query": {
          "field": "title_vector",
          "vector": [0.021, -0.043, ...],
          "k": 10
        }
      }
    }
  }'
```

## RRF (Reciprocal Rank Fusion)

Results from both search methods are merged using RRF:

```
score = 1 / (60 + rank_bm25) + 1 / (60 + rank_vector)
```

### RRF Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `k` | 60 | Constant added to each rank to smooth scores |
| `weights` | `[0.5, 0.5]` | Relative weight for each search method |

```bash
curl -X POST http://127.0.0.1:8080/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "hybrid": {
        "text_query": { "match": { "title": "deep learning" } },
        "vector_query": { "field": "title_vector", "vector": [...], "k": 10 },
        "rrf": {
          "k": 60,
          "weights": [0.6, 0.4]
        }
      }
    }
  }'
```

## Best Practices

1. **Tune RRF k**: Start with 60, increase if one method dominates, decrease if you want more diversity
2. **Normalize scores**: Ensure both methods produce comparable score ranges
3. **Filter after fusion**: Apply post-query filters to the merged result set
4. **Test with your data**: The optimal balance depends on your corpus and query types
