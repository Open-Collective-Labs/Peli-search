# Go SDK

## Installation

```bash
go get github.com/Open-Collective-Labs/Peli-search/sdk/go/pelisearch
```

## Quick Start

```go
package main

import (
    "context"
    "log"

    "github.com/Open-Collective-Labs/Peli-search/sdk/go/pelisearch"
)

func main() {
    client := pelisearch.NewClientFromURL("http://localhost:7700")
    ctx := context.Background()

    if _, err := client.CreateIndex(ctx, "products"); err != nil {
        log.Fatal(err)
    }

    _, err := client.AddDocument(ctx, "products", "p1", map[string]interface{}{
        "title": "Widget",
        "price": 9.99,
    })
    if err != nil {
        log.Fatal(err)
    }

    q := "widget"
    results, err := client.Search(ctx, "products", &pelisearch.SearchRequest{Q: &q})
    if err != nil {
        log.Fatal(err)
    }
    log.Printf("hits: %d", len(results.Hits))
}
```

All methods accept `context.Context` as the first argument for cancellation and timeouts.

## Searching

```go
q := "wireless mouse"
results, err := client.Search(ctx, "products", &pelisearch.SearchRequest{Q: &q})

// DSL match
results, err := client.Search(ctx, "products", &pelisearch.SearchRequest{
    Query: map[string]interface{}{
        "match": map[string]string{"title": "keyboard"},
    },
})
```

## Filtering

```go
filter := "category = electronics AND price < 100"
q := "keyboard"
results, err := client.Search(ctx, "products", &pelisearch.SearchRequest{
    Q:      &q,
    Filter: &filter,
})
```

## Aggregations

```go
q := "engineer"
results, err := client.Search(ctx, "jobs", &pelisearch.SearchRequest{
    Q:      &q,
    Facets: []string{"company", "location"},
})
fmt.Println(results.FacetDistributions)
```

## Error Handling

API errors return `*pelisearch.APIError` with `Status` and `Message`:

```go
_, err := client.GetIndex(ctx, "missing")
if apiErr, ok := err.(*pelisearch.APIError); ok {
    log.Printf("HTTP %d: %s", apiErr.Status, apiErr.Message)
}
```

Wrap calls with context deadlines:

```go
ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
defer cancel()
err := client.Health(ctx)
```
