# The APIs are built on their first request

**Number:** next free.

**Status:** Accepted, 2026-10-31. Follows "The APIs are built beside the first answer", whose first decision it replaces.

## Context

`khalid serve` built the Content API and Admin API router on a blocking thread right after the routes were wired, whether or not anyone would call the APIs. With every Admin API desk of the final bundle (integration 19), that router and its `OpenAPI` document were the largest thing an idle server held for them: a `dhat` profile of the 1,000-post site, 45 s after start, held 22.8 MB live, of which 7.2 MB was the document. The idle reading was 59.74 MB here and about 66 MB in CI, against the 55 MB budget.

The document was held more than once: `khalid_api::api::finish` served it as a `Json` of the struct, and the router copies a route's handler, with the whole document inside, as it is put together.

## Decision

- **The APIs are built when the first request under `/api/` arrives** (`late_api::router`). That request starts the build on a blocking thread and waits for it; requests that come while it runs wait for the same build, and the build finishes even if the request that started it goes away. Every other request goes to the site as before. A build that panics still answers 503 to the APIs and leaves the pages up.
- **The document is served as JSON bytes made once.** The handler holds shared bytes, so copying the route copies a pointer. The response is byte for byte the same, with the same `Content-Type`.

## Consequences

- A server nobody calls the APIs on never builds them, and holds none of it.
- Idle memory on the 1,000-post site fell from 59.86 to 61.40 MB to 50.63 to 51.51 MB in alternating runs, and after the APIs were asked it stays about 7.5 MB lower than before (docs/standards/performance.md, "What an idle server holds").
- The first API request after a start waits for the build, about 8.6 ms. The admin's first screen pays it once; nothing else does. `GET /api/openapi.json` went from 7.5 to 8.7 ms to 0.33 ms, since it is no longer written out on each request.
- Background work the APIs' desks start is per request, so nothing is delayed by the change. The Content API's store is still built at start by the content feed, which the webhooks, `IndexNow` and search need.
- Any new API route must still sit under `/api/`, or it must be merged outside `late_api`.
