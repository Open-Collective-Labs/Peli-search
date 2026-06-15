# SDK Integration Tests

Runs all official SDKs against a live PeliSearch server.

## Prerequisites

- Node.js 18+
- Python 3.10+
- Go 1.21+
- Rust toolchain

## Run

```bash
./sdk/tests/run.sh
```

The script builds `pelisearch-server` if needed, starts it on a dynamic port, and runs tests for JavaScript, Python, Go, and Rust.

Environment variables set during the run:

| Variable | Description |
|----------|-------------|
| `PELISEARCH_TEST_URL` | Base URL of the test server |
| `PELISEARCH_TEST_PORT` | Port number |
| `PELISEARCH_TEST_DATA_DIR` | Temporary data directory |

## Test Coverage

- Index management (create, list, get, delete)
- Document operations (add, get, bulk, delete)
- Search (legacy `q` and DSL match)
- Aggregations / facets
- Recovery (data persists across server restart)
