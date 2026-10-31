# The Analytics screen, 2026-10-31

The founder said yes in advance, on 2026-10-27, to every admin mockup in this phase. This records that yes for the Analytics screen, so the screen can be built without stopping for a review.

## What was agreed

- The mockup is `docs/design/admin-mockups/analytics.html`, beside the other admin mockups. CLAUDE.md names `docs/mockups/`; every admin mockup already lives in `docs/design/admin-mockups/`, so this one does too.
- The screen is "Analytics" in the side menu, after Redirects. It reads three Admin API routes, all needing `settings:write`, because views are visitor data:
  - `GET /api/admin/v1/analytics/visitors/?days=` for the page views per day, per source and per page (KH-GRO-4);
  - `GET /api/admin/v1/analytics/ai-visitors/?days=` for the readers each AI assistant sent (GEO M1);
  - `GET /api/admin/v1/analytics/uncited/?days=` for the pages AI bots read but no assistant cited (GEO M2).
- One focus point: the page views of the period against the period before, with the one blue button that changes the period. The choices are 7, 28 and 90 days; 28 is the default. "Read but never cited" covers the same days, since the crawl log keeps 90.
- The chart of each day is an SVG with a written summary, and a table of each day behind "Show each day as a table", so it reads without seeing it.
- Every term is defined where it first appears: page view, AI assistant, AI bot, cited.

## Left for later

- Bots per AI tool, which waits for C1 and the D2 address lists.
- Search Console numbers on this screen (G43 and M3).
