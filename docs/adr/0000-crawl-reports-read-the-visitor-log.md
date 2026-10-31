# 0000. Crawl reports and alerts read the visitor log and its crawled pages

**Number:** next free. **Status:** Proposed, 2026-10-31. GEO C2 to C5, decisions D2 and D5 in `docs/decisions/2026-10-28-geo-build-list.md`.

## Context

C2 to C5 ask for a first-crawl timer, crawl health, crawl alerts and a bot error report. They need C1, the visitor log, which keeps one row per visit for 7 to 14 days and is off until the founder's lawyer reads its design (D5). The research's "Do not build" list says no bot counts from the claimed name alone. The source rule allows SEO and GEO behaviour only from Steve Toth and Koray Tugberk Gubur; neither gives numbers for how fast a post should be crawled or how long a crawler may stay away.

## Decision

- **Only a checked crawler counts.** A visit counts as Googlebot only when its address is on Google's list. A visit from another address that uses the name is shown apart, as "pretending", and never in the counts. Crawlers with no list (Anthropic's, Apple's) are shown as unchecked.
- **A small table outlives the visit rows.** `crawled_pages` keeps the first and last fetch of each page by each checked crawler, answered with the page (2xx or 304). It is written in the same save as the visits, holds nothing about any person, and is kept when the visits are deleted. Without it the first crawl of a month-old post would be gone.
- **The first-crawl timer covers posts that went live after the first crawl the log saw.** An older post's first crawl may have come before the log was on, so it is left out rather than shown wrong.
- **Reports cover search and AI crawlers.** SEO tools, link previews and other bots are left out of health and errors.
- **Two alerts, with Khalid's own thresholds.** `crawl-stopped`: Googlebot or Bingbot, the two search engines that publish their addresses, has not fetched a page for 7 days; it clears when the crawler is back. `crawler-errors`: a crawler got a 5xx in the last hour, keyed by hour like the 404 spike. A 4xx is left to the 404 alert and `khalid crawl errors`. The week is a choice, not SEO advice.
- **A command line first, `khalid crawl first|health|errors`, with `--json`.** No Admin API route or screen yet: a screen needs a mockup and the founder's yes, and the route can reuse the same functions in `khalid-analytics`.
- **Everything stays off with the log.** The commands answer that the log is off; the alert round holds no log and skips the crawl checks.

## Consequences

- Sites that leave the log off, which is every site until D5 is done, get none of this.
- A crawler blocked so completely that it never reaches a page leaves no row, so `crawl-stopped` fires only for a crawler that came at least once while the log was on.
- Rows from Anthropic's crawlers are never counted as checked until a published list is added.
