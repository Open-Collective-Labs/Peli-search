# Storage Engine

## Write-Ahead Log (WAL)

Every document write is first appended to the WAL before being indexed. This ensures durability:

- On crash, unflushed documents are replayed from the WAL
- WAL entries are fsynced periodically (configurable interval)
- WAL is truncated after successful segment flush

### WAL Configuration

```json
{
  "wal": {
    "flush_interval_ms": 1000,
    "buffer_size_kb": 64,
    "max_entries": 10000
  }
}
```

## Segments

Segments are immutable data files that store indexed documents:

- **Append-only**: documents are added, never modified in place
- **Immutable**: once written, segments are never updated
- **Self-contained**: each segment holds its own inverted index, stored fields, and DocValues

### Segment Structure

```
segment_N.sg
├── Segment Header           # Magic bytes, version, metadata
├── Term Dictionary          # Sorted terms → posting list pointers
├── Posting Lists            # doc_id, term frequency, positions
├── Stored Fields            # Original document values
├── DocValues                # Column-oriented field storage
└── Segment Footer           # Checksum, offsets
```

## Merging

Small segments are merged into larger ones in the background:

- **Tiered merge policy**: merges segments of similar size
- **Deletion**: removed documents are garbage-collected during merge
- **Scheduling**: merge runs when segment count exceeds threshold

### Merge Benefits

- Fewer segments → faster search (fewer files to scan)
- Removes deleted documents → reclaims space
- Rewrites indexes → better compression

## Durability

| Mechanism | Purpose |
|-----------|---------|
| WAL | Crash recovery |
| Fsync | Data integrity |
| Checksums | Corruption detection |
| Atomic rename | Consistent segment commits |
