# 0000. Server analytics are counted on the server, in a crate of their own

**Number:** next free. **Status:** Proposed, 2026-10-31. Gap audit 2 brief G42, KH-GRO-4.

## Context

KH-GRO-4 asks for analytics with no cookies, counted on the server so they work with scripts blocked, with readers from ChatGPT, Perplexity, Gemini, Copilot and Claude grouped as AI search, and a crawl log of which bots fetched which page and when. `data/site.db` already has the visitor table `analytics_counts` (one row per day, page and referrer group), unused. The 404 log (ADR "stored redirects answer after pages") already shows how `khalid serve` records a request without making it wait for SQLite.

## Decision

- **A crate of its own, `khalid-analytics`.** It sorts a referrer into a group and names a bot from its user agent, and holds the tally the server keeps between writes. It knows nothing about HTTP, SQL or files, as `khalid-comments` and `khalid-newsletter` do not.
- **Nothing about the reader is kept.** No cookie, no address, no user agent, no identifier and no referring address: a view is the UTC day, the page path and one of six groups (`direct`, `internal`, `search`, `social`, `ai`, `other`). So there is no consent banner to show, and nothing a reader could ask to delete.
- **AI search is told apart by the referring host, and by `utm_source`.** ChatGPT adds `utm_source=chatgpt.com` to its links, which also catches readers whose browser sends no referrer. `gemini.google.com` is checked before Google search.
- **A bot is what its user agent says.** Known bots are named (Googlebot, Bingbot, GPTBot, ClaudeBot, PerplexityBot and about forty more), and anything else that calls itself a bot, crawler or spider, a common HTTP library, a headless browser, or a request with no user agent is "Other bot". A bot's request is never a view. Checking that Googlebot is really Google takes a reverse DNS lookup per address; that is left out, and the crawl log says what each bot claimed to be.
- **Counts are tallied in memory and written in batches**, off the request path, so a busy page costs one map update per view rather than one SQLite write.

## Consequences

- The counts cannot tell unique readers apart. That is the price of keeping nothing about them.
- A bot that copies a browser's user agent counts as a reader. Server logs in any tool share this.
- The last few seconds of counts before a hard stop may not be written.
