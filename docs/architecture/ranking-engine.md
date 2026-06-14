# Ranking Engine

## BM25 Implementation

BM25 is the core ranking algorithm. The implementation follows the standard formula:

```
score(D, Q) = Σ(t ∈ Q) IDF(t) · TF(t, D) · (k1 + 1) / (TF(t, D) + k1 · (1 - b + b · |D| / avgdl))
```

### IDF Computation

```
IDF(t) = ln(1 + (N - n(t) + 0.5) / (n(t) + 0.5))
```

Where:
- `N`: total number of documents
- `n(t)`: number of documents containing term `t`

### Implementation

```rust
pub struct Bm25Scorer {
    pub k1: f32,          // default: 1.2
    pub b: f32,           // default: 0.75
    pub avg_field_length: f32,
    pub doc_count: u64,
    pub total_term_freq: HashMap<Term, u64>,
    pub doc_freq: HashMap<Term, u64>,
}
```

### Precomputed Values

To avoid repeated computation, per-segment precomputed values are cached:

- `avg_field_length` for each text field
- `IDF(t)` for each term (updated when new segments are added)

## Scoring Pipeline

```
Query Tree
    │
    ▼
Query → Term → Lookup IDF → Lookup Postings → Compute BM25
    │              │                │
    ▼              ▼                ▼
Phrase → Proximity Scoring → BM25 + Proximity Boost
    │
    ▼
Boolean → Combine Must/Should/Filter Scores
    │              │
    ▼              ▼
Match → Combine Fields → Weighted Sum
```

### Phrase Scoring

Phrase queries add a proximity component:

```
phrase_score = BM25 * (1 + proximity_boost)
proximity_boost = max(0, 1 - distance / max_distance)
```

### Boolean Query Scoring

| Clause Type | Score Contribution |
|-------------|-------------------|
| `must` | Sum of matching clause scores |
| `should` | Sum of matching clause scores (min_should_match) |
| `filter` | 0 (no score contribution) |
| `must_not` | 0 (no score contribution) |

## Custom Scoring

Users can provide custom similarity configurations:

```json
{
  "similarity": {
    "bm25": {
      "k1": 1.2,
      "b": 0.75
    }
  }
}
```

Additional scoring models can be implemented via the `Similarity` trait:

```rust
pub trait Similarity {
    fn score(&self, term_freq: u32, doc_len: u32, avg_doc_len: f32, idf: f32) -> f32;
    fn idf(&self, doc_freq: u64, doc_count: u64) -> f32;
}
```
