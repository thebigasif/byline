# The APIs are built beside the first answer

**Number:** next free.

**Status:** Accepted, 2026-10-31. Follows "khalid serve answers before the full render".

## Context

`MIMALLOC_PURGE_DELAY=0` keeps an idle server inside its 55 MB budget, but it makes every allocation-heavy step slower, and the cold start to the first byte went over its 110 ms limit in 10 of 16 CI runs with it (PR #704). Timing marks in `khalid serve` on the 1,000-post site showed three steps in front of the first answer that the first answer does not need: the stylesheet's Brotli and gzip copies (11 ms, thrown away by the server), the Content API and Admin API router (8 to 9 ms, mostly utoipa assembling the OpenAPI document), and the full render, which started before those routers were built and competed with them.

## Decision

- **The served stylesheet is built without compressed copies.** `khalid_render::stylesheet_text` and `Asset::hashed_name` give the same text and the same address `khalid build` uses; the server compresses it with its pages after it answers, as before.
- **The API router is built on a blocking thread once the other routes are wired** (`late_api::router`). Every API route sits under `/api/`, so a front router with no routes of its own sends each request under `/api/` to the APIs, holding it until they are built, and every other request to the rest of the site. It adds no route of its own, so a handler sees the request's path parameters exactly as before. An address under `/api/` the APIs lack goes to the rest of the site, as it did when the routers were merged up front. A build that panics answers 503 to the APIs and leaves the pages up.
- **The full render starts once the routes are ready**, right before the server accepts its first connection.

## Consequences

- Locally, the server is ready to answer about 20 ms sooner on the 1,000-post site, and the first request is answered within a fraction of a millisecond of that.
- An API request in the first few milliseconds after start waits for the router, as a request for a post already waits for the full render.
- Any new API route must sit under `/api/`, or it must be merged outside `late_api`.
