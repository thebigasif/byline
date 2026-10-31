# 0000. MCP tools come from the OpenAPI document

**Number:** next free.
**Status:** Proposed, 2026-10-31. Brief G26 in `docs/audits/2026-10-27-gaps-2.md`, KH-API-5.

## Context

KH-API-5 asks for an MCP server that exposes every operation. `khalid mcp` had five tools written by hand, so every new route needed a sixth by hand, and the Admin API is still growing (G24). The router already describes every operation, its parameters and its body in the `OpenAPI` document it returns beside itself (BL-API-4), and a contract test keeps that document true to the routes.

## Decision

- **One tool per operation, made at start from the router's own document.** The name is the `operationId`, the description is the summary, the arguments are the path, query and header parameters by name, and the request body is `body`. Schemas the body names are copied under `$defs`, so each input schema stands alone. `Accept-Version` is left out: the API answers with its current version.
- **Body kinds follow the document.** A JSON body is sent as written, a `text/*` body is a string, and any other body, such as a media upload, is a list of numbers from 0 to 255, as the document's schema for it says.
- **A parameter the document places in the path, where the address has no slot for it, goes in the query**, because that is where the route reads it (`list_misses`, `explain_redirect`). Fixing those two annotations belongs to the API's owner.
- **Only operations a key may call are tools.** An operation whose security names `admin_key` takes the server's key; one that names only `content_key` takes the content key the server makes for itself. Signing in, passkeys, password changes, signing out and ownership transfer take no key or only a browser session, so they are not tools: an agent acts with a key and never handles a person's password.
- **The hand-written tools go**, except two that are not operations: `publish_post`, which is `edit_admin_post` with only the status, and `lint_post`, which stays until a lint route exists.
- **`khalid mcp` mounts every route `khalid serve` does**, through `api::router_for_serve`, so its document is the one serve answers. Staff accounts and webhooks are read from the site's database, as serve reads them. Comments, the media library and stored redirects are held in memory by a running `khalid serve`, which would not see a change made by another process, so `khalid mcp` keeps their routes on the stores that answer that the site has none, until serve and mcp share one process or reload those stores.
- **`crates/khalid/tests/agents.rs` reads `sdk/typescript/openapi.json`**, which `serve_api.rs` proves is the served document, and fails naming every operation a key may call that `khalid mcp` does not list.

## Consequences

- A new route is a tool the day it lands, with no MCP code. A changed parameter changes its tool.
- Tool names changed: `list_posts`, `read_post`, `create_draft` and `edit_post` became `browse_posts`, `read_admin_post`, `create_admin_post` and `edit_admin_post`. An agent set up with the old names reads the new ones from `tools/list` and `khalid agents`.
- Writing a post takes the API's own body shape, `{"posts": [{...}]}`, instead of flat arguments.
- The comment, media and redirect tools exist but say the site has none until the follow-up above.
