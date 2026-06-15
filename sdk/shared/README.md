# Shared Models

Single source of truth for PeliSearch SDK types across all languages.

## Layout

| Path | Purpose |
|------|---------|
| `openapi/pelisearch.yaml` | Full REST API specification (mirrors `docs/openapi.yaml`) |
| `schemas/` | JSON Schema definitions for core request/response types |
| `types.ts` | TypeScript reference used when implementing SDKs |
| `generators/` | Placeholder for future code generation tooling |

## Core Types

- `SearchRequest` / `SearchResponse` / `SearchHit`
- `IndexDefinition`
- `Document`
- `ErrorResponse`
- `QueryClause`

## Consistency Rules

1. Field names use **snake_case** in JSON (matching the REST API).
2. SDKs map to idiomatic naming in each language (camelCase for JS/Go/Rust, snake_case for Python).
3. When the API changes, update `openapi/pelisearch.yaml` and `schemas/` first, then sync SDK types.

## Generators

Future generators can produce language bindings from `openapi/pelisearch.yaml`:

```bash
# Example (not yet implemented):
# npx @openapitools/openapi-generator-cli generate -i sdk/shared/openapi/pelisearch.yaml -g typescript-fetch
```

Until generators are wired up, SDK types are maintained manually using `types.ts` and the JSON schemas as reference.
