# Go Search Examples

```go
// Simple query
q := "rust"
results, _ := client.Search(ctx, "articles", &pelisearch.SearchRequest{Q: &q})
fmt.Printf("Found %d results\n", results.Total)

// Field match with highlighting
highlight := true
results, _ = client.Search(ctx, "articles", &pelisearch.SearchRequest{
    Query: map[string]interface{}{
        "match": map[string]string{"title": "rust"},
    },
    Highlight: &highlight,
})
for _, hit := range results.Hits {
    fmt.Println(hit.Highlighted["title"])
}

// Filtered search
results, _ = client.Search(ctx, "products", &pelisearch.SearchRequest{
    Query: map[string]interface{}{
        "match": map[string]string{"name": "keyboard"},
    },
    Filters: []interface{}{
        map[string]interface{}{
            "range": map[string]interface{}{
                "price": map[string]float64{"gte": 50, "lte": 200},
            },
        },
    },
})

// Paginated and sorted
from := 0
size := 20
results, _ = client.Search(ctx, "products", &pelisearch.SearchRequest{
    Query: map[string]interface{}{
        "match": map[string]string{"category": "electronics"},
    },
    Sort: []pelisearch.SortField{
        {Field: "price", Order: "Asc"},
    },
    From: &from,
    Size: &size,
})
```
