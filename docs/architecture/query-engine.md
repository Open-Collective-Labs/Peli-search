# Query Engine Architecture

## Query Parsing

The JSON query DSL is parsed into an internal query tree:

```rust
pub enum Query {
    Match(MatchQuery),
    Phrase(PhraseQuery),
    Boolean(BooleanQuery),
    Term(TermQuery),
    Range(RangeQuery),
    All(MatchAllQuery),
}
```

### Parser Flow

```
JSON → serde_json::Value → QueryParser → Query Tree → Query Planner
```

## Query Execution

### Scorer

Each segment implements a `SegmentScorer` trait:

```rust
pub trait SegmentScorer {
    fn score(&self, query: &Query, collector: &mut dyn Collector) -> Result<()>;
}
```

### Execution Flow

1. **Segment selection**: Determine which segments to query (based on segment metadata)
2. **Query rewriting**: Optimize the query tree (e.g., constant scoring for filters)
3. **Scoring**: Each segment produces a stream of `(doc_id, score)` pairs
4. **Collection**: Results are collected and merged across segments
5. **Post-processing**: Filtering, sorting, pagination

### Parallel Execution

Segments are scored concurrently using a thread pool:

```
Segment 0 ─┤
Segment 1 ─┤──→ Collector (Merged Results)
Segment 2 ─┤
```

## Result Collection

The collector pattern allows different result-processing strategies:

| Collector | Purpose |
|-----------|---------|
| `TopDocs` | Collect top N results by score |
| `Count` | Count total matching documents |
| `Aggregate` | Compute aggregations during collection |
| `Filter` | Apply post-query filters |

```rust
pub trait Collector {
    fn collect(&mut self, doc_id: DocId, score: Score, segment: &SegmentReader);
    fn results(&self) -> Result<CollectorOutput>;
}
```

### Merging

Results from parallel segment scorers are merged using a binary heap:

1. Each segment produces a sorted stream of `(score, doc_id)`
2. The merger reads from all streams and emits the globally top-scored documents
3. Tie-breaking: `doc_id` is used for deterministic ordering
