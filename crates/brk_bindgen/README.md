# brk_bindgen

Code generation for BRK client libraries.

## What It Enables

Generate clients for Rust, JavaScript, Python, and LLMs from the OpenAPI specification and metric tree. Keeps every consumer in sync with available metrics and API endpoints without manual maintenance.

## Key Features

- **Multi-client**: Generates Rust, JavaScript, Python, and LLM clients
- **OpenAPI-driven**: Extracts endpoints and schemas from the OpenAPI spec
- **Metric catalog**: Includes all metric IDs and their supported indexes
- **Type definitions**: Generates types/interfaces from JSON Schema
- **Selective output**: Generate only the clients you need

## Core API

```rust,ignore
use brk_bindgen::{generate_clients, ClientOutputPaths};

let paths = ClientOutputPaths::new()
    .rust("crates/brk_client/src/lib.rs")
    .javascript("modules/brk-client/index.js")
    .python("packages/brk_client/brk_client/__init__.py")
    .llm("website")
    .llm("website_next");

generate_clients(&vecs, &openapi_json, &paths)?;
```

## Generated Clients

| Language | Contents |
|----------|----------|
| Rust | Typed API client using `brk_types`, metric catalog |
| JavaScript | ES module with JSDoc types, metric catalog, fetch helpers |
| Python | Typed client with dataclasses, metric catalog |
| LLM | Concise discovery and complete plain-text API references |

Language clients include:
- All REST API endpoints as typed functions
- Complete metric catalog with index information
- Type definitions for request/response schemas

The LLM client emits the standard discovery files and links to the live
OpenAPI and series endpoints instead of duplicating their catalogs.

## Built On

- `brk_query` for metric enumeration
- `brk_types` for type schemas
