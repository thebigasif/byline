# 0000. IndexNow from `khalid-seo`, and no sitemap pings

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 1, audit brief B-SEO-11 (KH-SEO-11).

## Context

Plan section 5.2 asks for "IndexNow on publish; sitemap ping to Google and Bing". IndexNow is a shared protocol: a site posts the addresses that changed to one endpoint, and Bing, Yandex, Seznam, Naver and Yep all hear about them at once. A search engine checks the request against a key file the site serves.

Both sitemap pings are gone. Google retired `google.com/ping` in June 2023 and it now answers 404 ([Google Search Central](https://developers.google.com/search/blog/2023/06/sitemaps-lastmod-ping)). Bing's `bing.com/ping` answers 410 Gone and Bing points sites to IndexNow instead ([ctrl.blog](https://www.ctrl.blog/entry/sitemap-ping-indexnow/)). Google now reads sitemaps from `robots.txt` and Search Console, and schedules recrawls from each entry's `lastmod`.

`khalid-seo`'s crate docs said it must not know about HTTP. The brief puts IndexNow in that crate and its sending through `khalid-outbound`.

## Decision

- **No sitemap pings.** Khalid sends none, because both endpoints are dead and a request that always fails only adds noise to the logs. Google is reached through the sitemap line in `robots.txt` and an accurate `lastmod`, which Khalid already writes. Bing is reached through IndexNow.
- **IndexNow lives in `khalid-seo`.** The crate builds the key file and each request's JSON body, and sends them with an `&Outbound` it is given (BL-RUN-8). It never names an HTTP client. Its rule becomes: it knows the IndexNow protocol, but not how HTTP is done.
- **Switched on by `[seo.indexnow]` in `site.toml`.** The table holds `key` and an optional `endpoint`, which defaults to the shared `https://api.indexnow.org/indexnow`. No table, nothing sent.
- **The key is a setting, not a secret.** IndexNow publishes it in the key file by design, so it is not wrapped in `Secret`. It must be 8 to 128 ASCII letters, digits and hyphens, checked when settings are read.
- **The key file sits in the blog's own folder**, at `/blog/<key>.txt` on the demo site, and every request names it in `keyLocation`. IndexNow accepts addresses inside the folder that holds the file, so a blog mounted under another site needs nothing from that site. An address outside the folder is refused before anything is sent, since search engines refuse the whole request for it.
- **Batches and retries.** One request carries at most 10,000 addresses, the protocol's limit, each address once. A request is retried after a pause when the answer is 429 or 5xx, or the request timed out or could not connect. A 400, 403 or 422 is not retried, because the same request would fail the same way.

## Consequences

- The publish path calls `IndexNow::send` with the addresses a publish changed, once it exists (audit item X5). Writing the key file into the build output and serving it comes with that wiring.
- If Google or Bing ever offer a working ping again, it is a new small part in this crate. Founder question raised in `docs/phases/phase-1.md`.
- `khalid-seo` now depends on `khalid-outbound` and on `tokio` for the pause between retries. Neither reaches the render path, which never sends anything.
