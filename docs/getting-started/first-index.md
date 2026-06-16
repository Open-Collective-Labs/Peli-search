# Your First Index

## What is an Index?

An index is a logical container that holds documents and their searchable data. It defines how fields are mapped, stored, and analyzed. Think of it as a table in a traditional database, but optimized for search.

## Creating an Index

Indexes are created via the REST API.

**REST API:**

```bash
curl -X POST http://127.0.0.1:7700/indexes \
  -H "Content-Type: application/json" \
  -d '{
    "name": "books"
  }'
```

**Embedded (Rust):**

```rust
use pelisearch_core::engine::SearchEngine;

let mut engine = SearchEngine::new();
engine.create_index("books")?;
```

## Adding Mappings

After creating an index, you can add fields with specific types using the embedded API:

```rust
use pelisearch_core::schema::{Field, FieldType, Mapping};

let mapping = Mapping::new(vec![
    Field::new("title", FieldType::Text, true),
    Field::new("author", FieldType::Keyword, false),
    Field::new("published_year", FieldType::Integer, false),
    Field::new("price", FieldType::Float, false),
]);
engine.create_index_with_mapping("books", mapping)?;
```

Supported field types: `Text`, `Keyword`, `Integer`, `Float`, `Boolean`.

## Index Lifecycle

1. **Create** — Define the index name
2. **Write** — Index documents
3. **Refresh** — Make documents searchable
4. **Merge** — Background segment compaction
5. **Delete** — Remove index when no longer needed
