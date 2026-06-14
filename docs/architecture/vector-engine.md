# Vector Engine Architecture

> **Phase 2 feature** — this document describes the planned implementation.

## HNSW Index

Vector search uses a Hierarchical Navigable Small World (HNSW) graph for approximate nearest neighbor (ANN) search.

### Graph Structure

```
Layer 3:    [A]──────────[B]         ← Sparse (few nodes)
             │            │
Layer 2:    [A]──[C]─────[B]──[D]    ← Increasing density
             │    │       │    │
Layer 1:    [A]──[C]──[E]─[B]──[D]   ← Dense (all nodes)
             │    │    │   │    │
Layer 0:    [A]──[C]──[E]─[B]──[D]   ← Full graph
```

### Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `m` | 16 | Maximum number of connections per node per layer |
| `m_max_0` | 32 | Maximum connections in layer 0 |
| `ef_construction` | 200 | Dynamic candidate list size during construction |
| `ef_search` | 100 | Dynamic candidate list size during search |

### Construction

1. Each vector is inserted sequentially
2. Entry point is the topmost layer
3. Greedy search finds the closest neighbors at the current layer
4. Node is connected to its nearest neighbors (up to `m`)
5. Node randomly assigned to upper layers (exponentially decreasing probability)

### Search

1. Start at the entry point (top layer)
2. Greedy traverse to find closest node at current layer
3. Descend to next layer, repeat
4. At layer 0, expand search with ef_search candidates
5. Return top-k results

## Similarity Metrics

### Cosine Similarity

```
cosine(A, B) = A · B / (|A| * |B|)
```

Range: [-1, 1], where 1 is most similar.

### Dot Product

```
dot_product(A, B) = Σ(A_i * B_i)
```

Range: unbounded, higher is more similar. Not normalized — vector magnitude matters.

### Euclidean Distance

```
euclidean(A, B) = sqrt(Σ(A_i - B_i)²)
```

Range: [0, ∞), where 0 is identical. Converted to a similarity score: `1 / (1 + distance)`.

## Quantization

To reduce memory usage, vectors can be quantized:

| Scheme | Bytes per Dimension | Memory Savings |
|--------|-------------------|----------------|
| f32 (full) | 4 | — |
| f16 (half) | 2 | 50% |
| i8 (scalar) | 1 | 75% |
| PQ | < 1 | 90%+ (lossy) |
