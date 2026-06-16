# Best Practices

## Performance

**DO:**

- Use bulk inserts (`bulkAddDocuments`) instead of individual `addDocument` calls
- Always paginate with `from`/`size`
- Cache repeated searches client-side
- Create indexes once at startup, not per-request
- Prefer field-specific `match` queries over `q` for relevance

**DON'T:**

- Fetch all documents in one request (no unbounded search)
- Create or delete indexes on every operation
- Run wildcard or empty searches on every keystroke
- Reindex unchanged documents

## Search Quality

- Boost important fields implicitly by using `match` on the most important field as the primary query
- Use `term` for exact matches (category, status, tags)
- Use `range` for numeric/date filters
- Combine query clauses via `filters` for pre-filtering before scoring
- Enable `highlight: true` when results need explanation

## Error Handling

All SDKs throw typed errors:

```typescript
import { PeliSearchError, isPeliSearchError } from "@pelisearch/client"

try {
  await client.getIndex("missing")
} catch (err) {
  if (isPeliSearchError(err)) {
    console.error(err.status, err.message)
  }
}
```

```python
from pelisearch import PeliSearchError

try:
    client.get_index("missing")
except PeliSearchError as err:
    print(err.status, err)
```

```go
_, err := client.GetIndex(ctx, "missing")
if apiErr, ok := err.(*pelisearch.APIError); ok {
    log.Printf("HTTP %d: %s", apiErr.Status, apiErr.Message)
}
```

```rust
match client.get_index("missing").await {
    Err(pelisearch::PeliSearchError::Http { status, message }) => {
        eprintln!("HTTP {status}: {message}");
    }
    Err(e) => eprintln!("{e}"),
    Ok(_) => {}
}
```

## HTTP Connection Management

Python SDK supports context manager:

```python
with PeliSearch("http://localhost:7700") as client:
    client.health()
```

Go SDK supports `context.Context` for timeouts:

```go
ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
defer cancel()
err := client.Health(ctx)
```

## Agent Workflow

When implementing search for a user:

1. Create the required indexes
2. Bulk-insert documents
3. Use `match` queries for text search
4. Apply `filters` for structured filtering
5. Use `sort` for ordering
6. Paginate with `from`/`size`
7. Enable `highlight` when relevant
