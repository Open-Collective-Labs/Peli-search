# Indexes

## Index Lifecycle

```
Create → Open → Write → Refresh → Search → Close → Delete
                      ↘ Merge ↙
```

1. **Create**: Define index name, mappings, and settings
2. **Open**: Allocate resources and load metadata
3. **Write**: Accept documents for indexing
4. **Refresh**: Make newly indexed documents visible to search
5. **Search**: Query documents
6. **Close**: Release resources
7. **Delete**: Remove index data from disk

## Index Organization

On disk, each index is stored as a directory:

```
data/
└── my_index/
    ├── meta.json          # Index metadata and settings
    ├── segments.json      # Active segment list
    └── segments/
        ├── _0.sg          # Segment data
        ├── _0.ti          # Term index
        ├── _0.frq         # Term frequencies
        ├── _0.prx         # Positions
        ├── _1.sg
        └── ...
```

## Naming Conventions

- **Index names** must be lowercase, 1-64 characters
- Allowed characters: `a-z`, `0-9`, `_`, `-`
- Must start with a letter or underscore
- Example: `my_index`, `products-v2`, `log_stream_2024`

## Index Limits

| Limit | Default | Maximum |
|-------|---------|---------|
| Indexes per server | 100 | 1000 |
| Fields per index | 256 | 1024 |
| Documents per index | Unlimited | — |
