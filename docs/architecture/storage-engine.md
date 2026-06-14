# Storage Engine Architecture

## WAL Design

The Write-Ahead Log ensures durability. Every document write is appended before indexing.

### WAL Format

```
+----------------+----------------+----------------+-----+
| Entry Header 1 | Entry Body 1   | Entry Header 2 | ... |
+----------------+----------------+----------------+-----+
```

Each entry header contains:
- **Checksum** (4 bytes): CRC32 of the entry body
- **Length** (4 bytes): Size of the entry body
- **Flags** (1 byte): Deleted, committed, etc.

### WAL Lifecycle

1. Document arrives → appended to WAL
2. WAL buffer reaches threshold → fsynced
3. Memory buffer flushed to segment → WAL truncated
4. On restart → WAL replayed to rebuild in-memory state

### Configuration

```rust
pub struct WalConfig {
    pub flush_interval: Duration,    // default: 1000ms
    pub buffer_size: usize,          // default: 65536 (64KB)
    pub max_entries: usize,          // default: 10000
    pub fsync: bool,                 // default: true
}
```

## Segment Design

Segments are immutable, self-contained data files.

### Segment File Layout

```
┌──────────────────────┐
│     Magic Bytes       │  "PELI" (4 bytes)
│     Version           │  u32
│     Segment Metadata  │  doc_count, field_count, etc.
│     Term Dictionary   │  Sorted terms → posting list offset
│     Posting Lists     │  doc_id, term_freq, positions
│     DocValues         │  Column-oriented field data
│     Stored Fields     │  Original document values
│     Segment Footer    │  Checksum, field offsets
└──────────────────────┘
```

### Naming Convention

Segments are named sequentially: `_0.sg`, `_1.sg`, `_2.sg`, etc.

### Memory Mapping

Segments are memory-mapped for zero-copy access. The OS handles loading frequently accessed pages into memory and evicting cold pages.

## Compaction / Merging

### Tiered Merge Policy

Segments are grouped into tiers by size:

| Tier | Size Range | Max Segments Before Merge |
|------|-----------|--------------------------|
| 0    | < 1 MB    | 4 |
| 1    | 1–10 MB   | 4 |
| 2    | 10–100 MB | 3 |
| 3    | > 100 MB  | 2 |

### Merge Process

1. Select segments from a tier exceeding the threshold
2. Read all documents from selected segments
3. Merge sorted term dictionaries
4. Merge posting lists, deduplicating deleted documents
5. Write new segment
6. Atomically swap segment manifest
7. Remove old segments

### Deletion During Merge

Documents marked as deleted are excluded from the merged segment, reclaiming disk space.
