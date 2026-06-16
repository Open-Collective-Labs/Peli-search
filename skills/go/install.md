# Go SDK

## Install

```bash
go get github.com/Open-Collective-Labs/Peli-search/sdk/go/pelisearch
```

## Client

```go
import (
    "context"
    "github.com/Open-Collective-Labs/Peli-search/sdk/go/pelisearch"
)

client := pelisearch.NewClientFromURL("http://localhost:7700")

// With custom HTTP client:
client := pelisearch.NewClientFromURL("http://localhost:7700",
    pelisearch.WithHTTPClient(&http.Client{Timeout: 10 * time.Second}),
)

// With host/port:
client := pelisearch.NewClient("127.0.0.1", 7700)
```

## Indexing

```go
ctx := context.Background()

// Create index
client.CreateIndex(ctx, "products")

// Add document
client.AddDocument(ctx, "products", "p1", map[string]interface{}{
    "name":     "Wireless Mouse",
    "category": "electronics",
    "price":    29.99,
})

// Bulk add
client.BulkAddDocuments(ctx, "products", []map[string]interface{}{
    {"id": "p2", "fields": map[string]interface{}{"name": "Keyboard", "category": "electronics", "price": 89.99}},
})

// List indexes
indexes, _ := client.ListIndexes(ctx)

// Get index info
info, _ := client.GetIndex(ctx, "products")

// Delete document / index
client.DeleteDocument(ctx, "products", "p1")
client.DeleteIndex(ctx, "products")
```

## Searching

```go
// Simple text search
q := "mouse"
results, err := client.Search(ctx, "products", &pelisearch.SearchRequest{Q: &q})
for _, hit := range results.Hits {
    fmt.Println(hit.DocumentID, hit.Score)
}

// Field-specific match
results, _ = client.Search(ctx, "products", &pelisearch.SearchRequest{
    Query: map[string]interface{}{
        "match": map[string]string{"name": "keyboard"},
    },
})

// With filters, sort, pagination, highlighting
from := 0
size := 20
highlight := true
results, _ = client.Search(ctx, "products", &pelisearch.SearchRequest{
    Query: map[string]interface{}{
        "match": map[string]string{"name": "keyboard"},
    },
    Filters: []interface{}{
        map[string]interface{}{
            "term": map[string]string{"category": "electronics"},
        },
    },
    Sort: []pelisearch.SortField{
        {Field: "price", Order: "Asc"},
    },
    From:      &from,
    Size:      &size,
    Highlight: &highlight,
})
```

## Response

```go
type SearchResponse struct {
    Hits         []SearchHit            `json:"hits"`
    Aggregations map[string]interface{} `json:"aggregations,omitempty"`
    Total        int                    `json:"total"`
}
type SearchHit struct {
    Index       string              `json:"index"`
    DocumentID  string              `json:"document_id"`
    Score       float64             `json:"score"`
    Highlighted map[string]string   `json:"highlighted,omitempty"`
}
```

## Error Handling

```go
_, err := client.GetIndex(ctx, "missing")
if apiErr, ok := err.(*pelisearch.APIError); ok {
    log.Printf("HTTP %d: %s", apiErr.Status, apiErr.Message)
}
```
