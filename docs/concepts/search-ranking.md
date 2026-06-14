# Search Ranking

## BM25

Peli Search uses BM25 (Best Matching 25) as the default ranking algorithm. BM25 scores documents based on term frequency (TF) and inverse document frequency (IDF):

```
score(D, Q) = Σ(w_i ∈ Q) IDF(w_i) · TF(w_i, D) · (k1 + 1) / (TF(w_i, D) + k1 · (1 - b + b · |D| / avgdl))
```

### Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `k1` | 1.2 | Controls term frequency saturation |
| `b` | 0.75 | Controls length normalization |

- **k1** (0.0 to 3.0): Higher values increase the impact of term frequency. At 0, TF is ignored and only IDF matters.
- **b** (0.0 to 1.0): Higher values penalize longer documents more. At 0, no length normalization.

### Customizing BM25

```json
{
  "similarity": {
    "bm25": {
      "k1": 1.5,
      "b": 0.5
    }
  }
}
```

## Scoring Pipeline

```
Query → Tokenize → Lookup Terms → BM25 Score → Combine → Sort
```

1. **Tokenize**: Query text is analyzed into tokens
2. **Lookup**: Each token is looked up in the inverted index
3. **Score**: BM25 computes a score for each matching document
4. **Combine**: Multi-field queries combine scores (sum or max)
5. **Sort**: Results are sorted by score descending

## Ranking Factors

| Factor | Impact |
|--------|--------|
| Term frequency | More occurrences → higher score (sub-linear) |
| Inverse document frequency | Rare terms → higher weight |
| Field length | Shorter fields → higher weight per match |
| Query terms | More matching terms → higher score |
