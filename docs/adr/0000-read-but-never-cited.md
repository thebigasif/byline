# 0000. Read but never cited: AI bot reads without an AI reader

**Number:** next free. **Status:** Proposed, 2026-10-31. GEO task M2, KH-GRO-4.

## Context

GEO task M2, "read but never cited", has only its name in `docs/phases/phase-geo.md`; the research behind the GEO list is not in the repository. The build list's notes set two limits: an AI bot visit never counts as a citation, and no bot is counted from its claimed name alone. `khalid serve` already keeps the crawl log (which bot read which page, by day, with the last status) and the view counts by referrer group, where readers from ChatGPT, Perplexity, Gemini, Copilot and Claude are the `ai` group. Neither Steve Toth nor Koray Tugberk Gubur, the only sources for SEO and GEO behaviour here, says how to measure this from server logs, so the details below are engineering choices, not SEO advice.

## Decision

- **A citation is a reader sent by an AI assistant**, a view in the `ai` referrer group. That is the one sign of a citation the server can see. A bot's read never counts.
- **A read is a request from an AI bot answered with a success or "not modified"** that day. Redirects, 404s and errors are not reads. Requests for files (any last path part with a dot, such as `robots.txt`, `llms.txt` or an image) are not pages and are left out.
- **One period for both sides.** A page is listed when AI bots read it in the period and no AI reader came to it in the same period. The default is 30 days; `--days` takes 1 to 90, the days the crawl log keeps.
- **Pure logic in `khalid-analytics`** (`uncited`), so the Admin API and the dashboard can call the same function later. The command `khalid analytics uncited` reads `data/site.db` the way `khalid webhooks` does, without reading the content.
- **Claimed names, said plainly.** The report names bots by the user agent they sent and says anyone can copy one. Checking a bot against its company's published address list (D2) comes with the visitor log, C1; until then the list is a lead to look at, not a count to trust.
- **The fix it points to is the answer capsule** (W1, decision D3), already in the linter, rather than new advice.

## Consequences

- A page cited through an AI assistant that sends no referrer and no `utm_source` looks uncited. The report cannot see those readers.
- A bot pretending to be GPTBot puts a page on the list. The list says so; D2's address check removes it later.
- No Admin API route or admin screen yet. Both would call `uncited` with the same two lists.
