# Go Example Projects

## E-commerce Search

```go
// Index products
client.BulkAddDocuments(ctx, "products", []map[string]interface{}{
    {"id": "mbp", "fields": map[string]interface{}{"name": "MacBook Pro", "brand": "Apple", "category": "laptops", "price": 1999}},
    {"id": "xps", "fields": map[string]interface{}{"name": "Dell XPS 13", "brand": "Dell", "category": "laptops", "price": 1499}},
})

// Search with filters and sorting
from := 0
size := 10
results, _ := client.Search(ctx, "products", &pelisearch.SearchRequest{
    Query: map[string]interface{}{
        "match": map[string]string{"name": "laptop"},
    },
    Filters: []interface{}{
        map[string]interface{}{
            "term": map[string]string{"category": "laptops"},
        },
    },
    Sort: []pelisearch.SortField{
        {Field: "price", Order: "Asc"},
    },
    From: &from,
    Size: &size,
})
```

## Documentation Search

```go
// Index docs
client.BulkAddDocuments(ctx, "docs", []map[string]interface{}{
    {"id": "1", "fields": map[string]interface{}{"title": "Installation Guide", "content": "...", "section": "getting-started"}},
    {"id": "2", "fields": map[string]interface{}{"title": "API Reference", "content": "...", "section": "reference"}},
})

// Search with highlighting
highlight := true
results, _ = client.Search(ctx, "docs", &pelisearch.SearchRequest{
    Query: map[string]interface{}{
        "match": map[string]string{"content": "installation"},
    },
    Highlight: &highlight,
})
```
