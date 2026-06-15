# Code Generators

This directory is reserved for OpenAPI-based code generation scripts.

## Planned Workflow

1. Edit `../openapi/pelisearch.yaml` when the REST API changes.
2. Run generators to refresh SDK types (optional, manual sync today).
3. Run `sdk/tests/run.sh` to verify all SDKs against a live server.

## Supported Tools (future)

- [OpenAPI Generator](https://openapi-generator.tech/) — multi-language client stubs
- [openapi-typescript](https://github.com/drwpow/openapi-typescript) — TypeScript types from OpenAPI
- [datamodel-code-generator](https://github.com/koxudaxi/datamodel-code-generator) — Pydantic models from JSON Schema

Manual SDK maintenance is intentional for Phase 8 to keep clients lightweight and closely aligned with the REST API.
