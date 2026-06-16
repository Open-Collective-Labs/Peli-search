# Go Indexing Examples

```go
ctx := context.Background()

// Create index
client.CreateIndex(ctx, "articles")

// Add single document
client.AddDocument(ctx, "articles", "doc-1", map[string]interface{}{
    "title":   "Getting Started with Rust",
    "content": "Rust is a systems programming language...",
    "tags":    []string{"programming", "rust"},
})

// Bulk add documents
client.BulkAddDocuments(ctx, "articles", []map[string]interface{}{
    {"id": "1", "fields": map[string]interface{}{"title": "Rust Ownership", "content": "..."}},
    {"id": "2", "fields": map[string]interface{}{"title": "Rust Borrowing", "content": "..."}},
})

// Get / delete document
doc, _ := client.GetDocument(ctx, "articles", "doc-1")
fmt.Println(doc["title"])
client.DeleteDocument(ctx, "articles", "doc-1")
```
