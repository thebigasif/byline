# 0003. The API is the only door

**Status:** Accepted, 2026-10-26. From the founder's plan, section 5.12.

## Context

The CLI, the MCP server, the web admin, SerpDad's publish target and users' own scripts all need to create, change and read content. Four code paths would drift. Ghost's Admin and Content APIs are the pattern developers already know.

## Decision

Every operation is a function in a library crate, exposed once through the HTTP API (`/api/content/v1/`, `/api/admin/v1/`). The CLI, the MCP server and the admin are clients of those same functions, in process for the CLI and MCP, over HTTP for the admin. There is no operation the admin can do that the API cannot. The OpenAPI document is generated from the code and tested against the routes (BL-API-4).

## Consequences

- A new feature is not done until it is in the API, the OpenAPI document, the CLI and the MCP server. Usually that is one function and three thin wrappers.
- Auth, rate limits, validation and `updated_at` checks live in one place.
- Headless mode falls out for free: switch the theme off, keep the API.
- API changes follow the versioning rules in `docs/standards/api.md`.
