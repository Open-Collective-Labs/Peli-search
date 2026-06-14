# API Design

## REST API

### Base URL

```
http://127.0.0.1:8080
```

### Endpoints

#### Index Management

| Method | Path | Description |
|--------|------|-------------|
| `PUT` | `/indexes/{index}` | Create or update an index |
| `DELETE` | `/indexes/{index}` | Delete an index |
| `GET` | `/indexes/{index}` | Get index metadata |
| `GET` | `/indexes` | List all indexes |

#### Document Operations

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/indexes/{index}/documents` | Index a document |
| `POST` | `/indexes/{index}/documents/bulk` | Bulk index documents |
| `GET` | `/indexes/{index}/documents/{id}` | Get document by ID |
| `DELETE` | `/indexes/{index}/documents/{id}` | Delete a document |

#### Search

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/indexes/{index}/search` | Search an index |

### Request/Response Format

All requests and responses use `application/json`.

#### Search Request

```json
{
  "query": {
    "match": { "field": "value" }
  },
  "filter": { ... },
  "sort": [{"field": "asc"}],
  "from": 0,
  "size": 10,
  "aggregations": { ... }
}
```

#### Search Response

```json
{
  "hits": [
    {
      "id": "doc_1",
      "score": 0.693,
      "document": { "field": "value" }
    }
  ],
  "total_hits": 42,
  "query_time_ms": 3,
  "cursor": "...",
  "aggregations": { ... }
}
```

### Error Format

```json
{
  "error": {
    "code": "index_not_found",
    "message": "Index 'movies' does not exist",
    "status": 404
  }
}
```

## Embedded SDK

### Rust

```rust
use peli_search::Index;

let mut index = Index::create("books")?;
index.add_document(json!({
    "title": "The Hobbit",
    "author": "J.R.R. Tolkien",
    "year": 1937
}))?;

let results = index.search()
    .match_query("title", "hobbit")
    .execute()?;
```

## SDK Contracts

### Common Interface Across All SDKs

```rust
// Create/Open index
Index::create(name) -> Index
Index::open(name) -> Index

// Document operations
index.add_document(document) -> Result
index.add_documents(batch) -> Result
index.get_document(id) -> Document
index.delete_document(id) -> Result

// Search
index.search() -> SearchBuilder
    .match_query(field, text) -> Self
    .phrase_query(field, text) -> Self
    .boolean_query(clauses) -> Self
    .filter(filter) -> Self
    .sort(field, order) -> Self
    .from(offset) -> Self
    .size(limit) -> Self
    .execute() -> SearchResult
```
