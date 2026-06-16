# Python Indexing Examples

```python
from pelisearch import PeliSearch

client = PeliSearch("http://localhost:7700")

# Create index
client.create_index("articles")

# Add single document
client.add_document("articles", "doc-1", {
    "title": "Getting Started with Rust",
    "content": "Rust is a systems programming language...",
    "tags": ["programming", "rust"],
})

# Bulk add documents
client.bulk_add_documents("articles", [
    {"id": "1", "fields": {"title": "Rust Ownership", "content": "..."}},
    {"id": "2", "fields": {"title": "Rust Borrowing", "content": "..."}},
])

# Get / delete document
doc = client.get_document("articles", "doc-1")
print(doc["title"])
client.delete_document("articles", "doc-1")
```
