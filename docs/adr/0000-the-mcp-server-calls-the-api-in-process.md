# 0000. The MCP server calls the API in process

**Number:** next free.
**Status:** Proposed, 2026-10-27. Brief B-API-5 in `docs/audits/2026-10-27-phase-1-gaps.md`, KH-API-5.

## Context

`khalid mcp` lets agents such as Claude Code work on a blog through the Model Context Protocol (plan sections 4.1 and 5.8). ADR 0003 says the MCP server is a client of the same functions the API calls, in process. The protocol has an official Rust crate, `rmcp`, and the API has routes with key checks, scopes, body checks, the lint publish gate and problem details that the MCP tools must not drift from.

## Decision

- **The protocol is written by hand** in `khalid-mcp`, over `serde_json` and `tokio`: one JSON-RPC 2.0 message per line on stdin and stdout, `initialize`, `ping`, `tools/list` and `tools/call`, and nothing else. It is under 200 lines. `rmcp` brings proc macros and `schemars` for the same few methods.
- **Versions.** The server speaks protocol versions 2025-11-25, 2025-06-18, 2025-03-26 and 2024-11-05, whose tool methods are the same. A client asking for another gets the newest. Batches are refused, as 2025-06-18 removed them.
- **Each tool sends the request an HTTP client would**, through the same `axum` router `khalid serve` mounts, without a socket. Key checks, scopes, body checks, the publish gate and problem details are therefore the API's own code. A tool answers with the API's JSON body. A problem comes back as a tool result marked as an error, with the same JSON, so the agent reads what, why and fix.
- **One key.** The server acts with the integration key or staff token in `KHALID_KEY`, set in the agent's settings so it never appears on a command line. It is checked once at start, and a missing or refused key stops the server with a message before it reads stdin. Every Admin API call then carries it, so each call is checked again as over HTTP.
- **Listing reads the Content API.** That API takes a content key, which a writing key outranks. The server makes a content key in memory at start, for its own requests only. It is never stored, printed or sent anywhere.
- **Calls run one at a time**, in the order they arrive.

## Consequences

- A new Admin API route becomes an MCP tool with a schema and a request, and no second implementation.
- An agent sees exactly the errors an HTTP client sees, including every lint finding when publishing is refused.
- Nothing listens on a port: the agent starts `khalid mcp` and talks to it over a pipe.
- A tool with no API route yet, such as linting one post, calls the function the route would call, and moves onto the route once it exists.
