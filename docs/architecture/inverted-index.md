# Inverted Index Architecture

## Overview

The inverted index maps terms to the documents that contain them. It is the core data structure enabling fast full-text search.

```
Term → [doc_id, positions...]
─────────────────────────────
brown    → (doc_2, [3]), (doc_5, [7])
fox      → (doc_2, [4]), (doc_3, [1])
quick    → (doc_2, [2]), (doc_7, [0])
the      → (doc_1, [0,5]), (doc_2, [1,8]), ...
```

## Tokenization

The text analysis pipeline converts raw text into searchable tokens:

```
Raw Text → Character Filter → Tokenizer → Token Filter → Tokens
```

### Built-in Analyzers

| Analyzer | Description |
|----------|-------------|
| `standard` | Lowercases, removes punctuation, splits on whitespace |
| `simple` | Lowercases, splits on non-letter characters |
| `whitespace` | Splits on whitespace only |
| `english` | Standard + English stemming + stop words removal |

```rust
pub struct Analyzer {
    pub tokenizer: Tokenizer,
    pub filters: Vec<TokenFilter>,
}
```

## Posting Lists

A posting list records every document and position where a term appears.

### Encoding

Posting lists use delta encoding and variable-byte compression:

```
doc_ids (delta-encoded):  [1, 3, 2, ...] → [1, 4, 6, ...]
frequencies:              [3, 1, 5, ...]
positions (per doc):      [[0, 5, 10], [3], [1, 7, 12, 15, 20], ...]
```

- Document IDs are stored as deltas (gap between consecutive docs)
- Both doc IDs and positions are encoded with variable-byte integers (VInt)
- Term frequencies use a single byte for common values (1-255)

### Skip Lists

Long posting lists include skip lists for faster intersection:

```
Level 2: doc_1000 ──── doc_5000 ──── doc_10000
Level 1: doc_100 ─ doc_500 ─ doc_1000 ─ ...
Level 0: doc_1, doc_2, ..., doc_100, ...
```

## Term Dictionary

The term dictionary maps terms to their posting list location:

### Structure

- **Terms** are stored sorted lexicographically
- **Block-based**: terms are grouped into blocks of 64
- **Block header** stores the first term and a bloom filter
- **Lookup**: binary search on block headers, then linear scan within block

### FST (Finite State Transducer)

The term dictionary uses an FST for memory-efficient prefix search:

- Supports prefix queries (`prefi*`)
- Supports fuzzy queries (Levenshtein automaton)
- Memory-efficient: ~1 byte per term on average
