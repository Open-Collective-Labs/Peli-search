## Goal
Complete the REST API with Swagger UI documentation, then add integration tests for all API operations.

## Constraints & Preferences
- OpenAPI spec served as JSON at `/openapi.json`, Swagger UI at `/docs`
- Integration tests live in `crates/pelisearch-server/tests/` with files `api_indexes.rs`, `api_documents.rs`, `api_search.rs`, `recovery.rs`
- Tests must cover Index CRUD, Document CRUD, Search (query/filters/sorting/aggregations), and recovery/persistence restart

## Progress
### Done
- **OpenAPI JSON endpoint** — `GET /openapi.json` serves the OpenAPI 3.1 YAML spec as pretty-printed JSON; parsed at compile time via `include_str!` → `serde_yaml` → `serde_json`; cached in `AppState.openapi_spec`
- **Swagger UI endpoint** — `GET /docs` serves an interactive Swagger UI HTML page from `docs/swagger-ui.html`
- **Middleware refactor** — separated into three files: `request_id.rs` (UUID per request + `X-Request-Id` header), `metrics.rs` (`request_count` + `total_latency_ns`), `logging.rs` (pure access logging); correct ordering: metrics → request_id → logging (outermost)
- **Request ID in logs** — logging middleware reads `X-Request-Id` from the response header set by the outer `request_id` middleware
- **Full documentation** — `docs/openapi.yaml` now documents all 9 paths including `/health`, `/ready`, `/metrics`, `/indexes`, `/indexes/{name}`, `/indexes/{name}/documents`, `/indexes/{name}/documents/bulk`, `/indexes/{name}/documents/{id}`, `/indexes/{name}/search`; includes `MetricSnapshot` schema
- **Integration tests** — 11 tests across 4 files covering Index CRUD (create/read/list/delete, duplicates, empty name, 404), Document CRUD (insert/retrieve/delete, bulk add, partial errors, duplicates), Search (legacy `q`, DSL match, no results, invalid query, index not found), Recovery (persistence across restart, data integrity after deletions)
- **Cross-process port safety** — `pick_port()` uses atomic counter (intra-process) + `flock` lock file (inter-process) to prevent port collisions between parallel test binaries; `fuser -k` cleans up orphaned servers; child-exit check prevents connecting to stale orphan servers
- **All 543+ tests pass** across workspace, 0 warnings, 0 failures

### In Progress
- (none)

### Blocked
- (none)

## Key Decisions
- OpenAPI YAML parsed to JSON once at server startup via `include_str!` + `serde_yaml` + `serde_json`, cached in `AppState` as `OpenApiSpec(Arc<String>)`
- `/docs` uses Swagger UI fetched from unpkg CDN (no bundling required)
- Middleware ordering: first `.layer()` wraps the router (innermost), subsequent layers wrap outward
- Integration tests run the server binary as a subprocess using `std::process::Command` for true end-to-end coverage
- Port allocation uses `AtomicU16` counter + cross-process `flock` to prevent collisions; `fuser -k` kills orphaned processes; child-exit check validates our server actually bound

## Relevant Files
- `crates/pelisearch-server/tests/common/mod.rs` — test helper: `start_server`, `stop_server`, `url`, `pick_port` with flock + atomic counter
- `crates/pelisearch-server/tests/api_indexes.rs` — Index CRUD (1 test)
- `crates/pelisearch-server/tests/api_documents.rs` — Document CRUD (3 tests)
- `crates/pelisearch-server/tests/api_search.rs` — Search API (5 tests)
- `crates/pelisearch-server/tests/recovery.rs` — Recovery/persistence (2 tests)
- `crates/pelisearch-server/Cargo.toml` — dev-deps: `reqwest` (json+blocking), `tempfile`, `libc`
