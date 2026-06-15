## Goal
Build official SDKs for JavaScript, Python, Go, and Rust to eliminate raw HTTP calls.

## Constraints & Preferences
- SDKs must be lightweight, closely mirror the REST API
- Consistent naming across all 4 languages
- Strong typing, good error handling, comprehensive type exports
- Semantic versioning, developer experience first

## Progress
### Done
- **SDK workspace** — `sdk/` directory with `javascript/`, `python/`, `go/`, `rust/`, `shared/`
- **JavaScript SDK** (`sdk/javascript/`) — TypeScript, `@pelisearch/client`, ESM, default host `http://localhost:7700`, modular `indexes`/`documents`/`search` internals, full typed client
- **Python SDK** (`sdk/python/`) — `pelisearch` package, `PeliSearch()` factory + `PeliSearchClient`, split modules (`indexes.py`, `documents.py`, `search.py`, `exceptions.py`), Python 3.10+
- **Go SDK** (`sdk/go/pelisearch/`) — split files (`index.go`, `document.go`, `search.go`), `context.Context` on all methods, `NewClientFromURL`, typed `APIError`
- **Rust SDK** (`sdk/rust/`) — split modules (`index.rs`, `search.rs`), async `Client`, `Client::from_url`, `SearchRequest: Default`
- **Shared models** — `sdk/shared/openapi/`, `schemas/`, `generators/`, `types.ts`
- **Examples** — `examples/ecommerce`, `examples/blog`, `examples/docs`, `examples/jobs` (JS + Python)
- **Documentation portal** — `docs/sdk/` with per-language guides (installation, quick start, searching, filtering, aggregations, error handling)
- **Integration tests** — `sdk/tests/run.sh` runs JS, Python, Go, Rust tests against live server
- **CI** — `.github/workflows/sdk.yml`

### In Progress
- (none)

### Blocked
- (none)

## Key Decisions
- Each SDK is a standalone package in its language's native format (npm, PyPI, Go module, cargo crate)
- Shared types documented in `sdk/shared/types.ts`, JSON schemas, and `docs/openapi.yaml`
- Naming convention: `createIndex`/`create_index`, `listIndexes`/`list_indexes`, `bulkAddDocuments`/`bulk_add_documents`
- SDK response types align with current API (`SearchHit` includes `index`; pagination fields optional until server adds them)
- All HTTP errors wrapped into typed errors with `status` field

## Relevant Files
- `sdk/javascript/src/client.ts` — `PeliSearchClient`
- `sdk/python/src/pelisearch/client.py` — `PeliSearchClient`, `PeliSearch()`
- `sdk/go/pelisearch/` — Go client with context support
- `sdk/rust/src/` — async Rust client
- `sdk/shared/` — OpenAPI mirror, JSON schemas, type reference
- `sdk/tests/run.sh` — integration test runner
- `docs/sdk/` — SDK documentation portal
- `examples/` — onboarding examples
- `.github/workflows/sdk.yml` — CI
