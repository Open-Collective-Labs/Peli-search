# Installation

## JavaScript / TypeScript

```bash
npm install @pelisearch/client
```

## Python

```bash
pip install pelisearch
```

## Go

```bash
go get github.com/Open-Collective-Labs/Peli-search/sdk/go/pelisearch
```

## Rust

```toml
[dependencies]
pelisearch = "0.1"
```

## Client Creation

### JavaScript

```typescript
import { PeliSearchClient } from "@pelisearch/client"

const client = new PeliSearchClient()
// Defaults to http://localhost:7700
// Custom host:
const client = new PeliSearchClient({ host: "http://localhost:7700" })
```

### Python

```python
from pelisearch import PeliSearch

client = PeliSearch("http://localhost:7700")
# Also supports host/port:
client = PeliSearch(host="127.0.0.1", port=7700)
```

### Go

```go
import "github.com/Open-Collective-Labs/Peli-search/sdk/go/pelisearch"

client := pelisearch.NewClientFromURL("http://localhost:7700")
```

### Rust

```rust
use pelisearch::Client;

let client = Client::from_url("http://localhost:7700")
    .expect("valid URL");
```

## Agent Guidance

- Default host is `http://localhost:7700`
- Use `PeliSearchClient` in JS, `PeliSearch` (factory) in Python, `Client` in Go/Rust
- All SDK methods are async (except Go accepts `context.Context`)
