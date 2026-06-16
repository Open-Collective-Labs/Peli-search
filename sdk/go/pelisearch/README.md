# PeliSearch Go SDK

Official Go client for [PeliSearch](https://github.com/Open-Collective-Labs/Peli-search).

## Installation

```bash
go get github.com/Open-Collective-Labs/Peli-search/sdk/go/pelisearch
```

Requires Go 1.22+.

## Quick Start

```go
import (
    "context"
    "github.com/Open-Collective-Labs/Peli-search/sdk/go/pelisearch"
)

func main() {
    client := pelisearch.NewClientFromURL("http://localhost:7700")
    ctx := context.Background()

    client.CreateIndex(ctx, "products")
    client.AddDocument(ctx, "products", "doc1", map[string]interface{}{
        "title": "Widget",
        "price": 9.99,
    })

    q := "widget"
    results, _ := client.Search(ctx, "products", &pelisearch.SearchRequest{Q: &q})
    for _, hit := range results.Hits {
        println(hit.DocumentID, hit.Score)
    }
}
```

## Documentation

See the [full SDK docs](https://github.com/Open-Collective-Labs/Peli-search) for detailed usage.

## License

MIT
