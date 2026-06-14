# Documents

## JSON Documents

Documents in Peli Search are JSON objects. Each document represents a single record with fields of various types.

```json
{
  "id": "doc_123",
  "title": "The Lord of the Rings",
  "author": "J.R.R. Tolkien",
  "year": 1954,
  "isbn": "978-0-618-00222-8",
  "tags": ["fantasy", "adventure"],
  "rating": 4.8
}
```

## Document ID

Every document must have a unique `id` field within its index. IDs can be:

- **String** — explicitly provided at index time
- **Auto-generated** — assigned as a UUID if omitted

## Field Types

| Type | JSON Representation | Example |
|------|-------------------|---------|
| text | string | `"The Matrix"` |
| keyword | string | `"sci-fi"` |
| integer | number (int) | `1999` |
| float | number (float) | `4.5` |
| boolean | bool | `true` |
| array | array | `["tag1", "tag2"]` |

## Storage

Documents are stored in immutable segments on disk:

- **Stored fields** — the original document content, retrievable in search results
- **Indexed fields** — tokenized and stored in the inverted index for searching
- **DocValues** — column-oriented storage for sorting, filtering, and aggregations

## Retrieval

Documents are retrieved via:

- **Search queries** — matched documents are returned with relevance scores
- **Get by ID** — direct lookup using the document ID
- **Scan** — iterate all documents in an index
