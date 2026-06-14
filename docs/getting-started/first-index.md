# Your First Index

## What is an Index?

An index is a logical container that holds documents and their searchable data. It defines how fields are mapped, stored, and analyzed. Think of it as a table in a traditional database, but optimized for search.

## Creating an Index

Indexes are created via the REST API or the embedded SDK.

**REST API:**

```bash
curl -X POST http://127.0.0.1:8080/indexes \
  -H "Content-Type: application/json" \
  -d '{
    "name": "books",
    "mappings": {
      "properties": {
        "title": { "type": "text" },
        "author": { "type": "keyword" },
        "published_year": { "type": "integer" },
        "price": { "type": "float" }
      }
    }
  }'
```

**Embedded (Rust):**

```rust
use peli_search::Index;

let mut index = Index::create("books")?;
index
    .add_text_field("title")?
    .add_keyword_field("author")?
    .add_integer_field("published_year")?
    .add_float_field("price")?;
```

## Index Settings

When creating an index, you can configure:

| Setting | Default | Description |
|---------|---------|-------------|
| `number_of_shards` | 1 | Number of shards |
| `refresh_interval` | 1s | How often indexed documents become visible |
| `write_buffer_size` | 64 KB | Buffer size for the write-ahead log |
| `segment_memory_mb` | 256 | Max memory per segment before flush |

### Example with Settings

```bash
curl -X POST http://127.0.0.1:8080/indexes \
  -H "Content-Type: application/json" \
  -d '{
    "name": "logs",
    "mappings": { ... },
    "settings": {
      "refresh_interval": "5s",
      "segment_memory_mb": 512
    }
  }'
```

## Index Lifecycle

1. **Create** — Define mappings and settings
2. **Write** — Index documents
3. **Refresh** — Make documents searchable
4. **Merge** — Background segment compaction
5. **Delete** — Remove index when no longer needed
