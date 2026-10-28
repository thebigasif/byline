# GEO build list answers, 2026-10-28

The research of 28 October 2026 (main at 1fef9e8) produced a build list for writing checks, freshness, page structure, crawling, measuring and the audit. The founder said on 2026-10-27: "you got my all yes. dont stop any work for my yes." These answers settle its open decisions. The tasks are in `docs/phases/phase-geo.md`.

## Source rule

SEO, GEO and AEO behaviour comes only from Steve Toth and Koray Tugberk Gubur. When neither covers something, the pull request says so and builds nothing on another source. Plain facts, such as a company's published crawler address list, are data, not SEO advice.

## Decisions

- **D1. AI crawlers by default.** Allow all, as today. `site.toml` offers one switch, "search yes, AI training no", that writes the matching `robots.txt` preset from the crawler list in `khalid-seo`.
- **D2. Crawler address lists.** Yes. The visitor log may read the address lists Google, Anthropic, OpenAI, Perplexity and Bing publish, fetched through `Outbound` and cached, to tell a real crawler from one that only claims the name.
- **D3. Answer capsule.** 40 to 60 words after a question-style heading, adjustable in `lint.toml`.
- **D4. Freshness windows.** The stale-post warning stays at 12 months. The review queue (F3) asks for a review every 6 months. Dates change only on a real update.
- **D5. Privacy review of the visitor log.** The visitor log (C1) is built with the privacy design in the list: no cookies, shortened addresses only, a daily-erased visitor code, raw rows kept 7 to 14 days. It ships switched off by default until the founder's lawyer has read the design. This is the one step only the founder can do.
- **D6. Build order.** First batch: S1, F1, P1, P2, P5, W1 to W4, A1. Then F2, F3, P3, P4, P6 to P9, W5 to W7, C1 to C8, M1 to M4.

## Notes

- P2 (the scripts-off check on every CI build) needs a change to `.github/workflows/`. The maintainers' GitHub tokens lack the workflow scope, so P2 waits for the founder to grant it.
- The "Do not build" list in the research stands: no different content for bots, no date changes without a real update, llms.txt never the main AI-search feature, no bot counts from the claimed name alone, and an AI bot visit never counts as a citation.
