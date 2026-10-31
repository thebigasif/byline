### Added

- Opt-in Google Search Console connection with your own Google account. Add `[search_console]` to `site.toml` and set `KHALID_GOOGLE_CLIENT_SECRET` and `KHALID_GOOGLE_REFRESH_TOKEN`; without them Khalid asks Google nothing.
- `khalid search-console check`, `report`, `post <slug>`, `index` and `email`: each post's clicks, impressions, click-through rate and position for the last 28 days, the searches behind a post, index status, the week's winners and losers, posts whose clicks fell three months in a row, missing and noindex pages that still get search traffic, a weekly email, and "Go deeper in SerpDad" links where a number asks a question Khalid cannot answer.
