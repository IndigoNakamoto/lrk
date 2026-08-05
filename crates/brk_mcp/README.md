# brk_mcp

`brk_mcp` is a thin, stateless, read-only MCP adapter for the BRK REST API. It
exposes the generated OpenAPI operations as MCP tools and forwards every tool
call to the configured public REST origin as a `GET` request.

## Caching model

`brk_mcp` does not cache API responses or retain MCP sessions. Point it at the
public Cloudflare-fronted REST origin so its upstream `GET` requests use the
existing Cloudflare cache:

```text
MCP client -> brk_mcp -> Cloudflare-cached REST API -> BRK server
```

Cloudflare does not need to cache the `/mcp` endpoint. The MCP catalog TTL is
only a standard client-side cache hint for the static discovery and tool-list
metadata.

## Run

From the workspace:

```sh
cargo run --bin brk_mcp -- bitview.space
```

Or run an installed binary:

```sh
brk_mcp bitview.space
```

A bare host tries HTTPS first and retries over HTTP only if the HTTPS request
fails at the transport layer. An explicit origin uses only that origin:

```sh
brk_mcp http://127.0.0.1:3110
```

The Streamable HTTP endpoint is `http://127.0.0.1:3111/mcp` by default. If that
port is unavailable, the server tries each port through `3211`. The server
supports MCP protocol version `2026-07-28`.

The REST URL or host is the only configuration. It must be an HTTP(S) origin
without a path, query, or credentials.

## Generated catalog

The tool catalog is generated from the canonical OpenAPI document by
`brk_bindgen` and embedded in the binary at compile time from
`generated/manifest.json`. Do not edit the generated manifest by hand; rerun
the BRK bindgen target after changing the API.
