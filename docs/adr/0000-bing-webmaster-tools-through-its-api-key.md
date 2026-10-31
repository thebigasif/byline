# 0000. Bing Webmaster Tools through its API key, and IndexNow on file updates

**Number:** next free. **Status:** Proposed, 2026-10-31. GEO phase, task C8.

## Context

Task C8 in `docs/phases/phase-geo.md` reads "Bing Webmaster Tools and IndexNow on updates", with no more detail. IndexNow on an updated date reached the publish door with F1: a live post edited through the API is sent to IndexNow. A live post whose file is edited on disk, by a text editor or a `git pull`, is not. Content files are the source of truth (ADR 0001), so that path matters as much as the door.

Bing Webmaster Tools has a public API ([Microsoft Learn](https://learn.microsoft.com/en-us/bingwebmaster/getting-started)). Its JSON endpoint is `https://ssl.bing.com/webmaster/api.svc/json/<Method>`, and it signs in with OAuth or with an API key in the `apikey` query parameter. The key belongs to a person and reads every site they verified. The calls a blog needs are `SubmitFeed` (a sitemap), `GetFeeds` (the sitemaps Bing has and their status), `GetCrawlStats` (six months of daily crawl counts) and `GetCrawlIssues` (addresses Bing had trouble with, as combined flags).

The source rule (decision file 2026-10-28) says SEO and GEO behaviour comes only from Steve Toth and Koray Tugberk Gubur. Neither covers the shape of this API or the details below. These are Microsoft's published facts, used as data, not as SEO advice.

## Decision

- **The API key, not OAuth.** OAuth needs a registered app, a redirect address and a token store, which a one-binary blog does not have. The key is read once from `KHALID_BING_API_KEY`, wrapped in `Secret`, hidden from `Debug`, and never written to a file or a log (BL-RUN-5). No key, no Bing calls.
- **The client lives in `khalid-seo`, beside IndexNow**, and sends through the `Outbound` it is given (BL-RUN-8). Because the key travels in the address, an error from the client never carries the HTTP library's error, whose text includes the address. It names the kind of failure only.
- **`khalid bing` is the door for a person.** `khalid bing sitemap` submits the site's sitemap index; `khalid bing report` prints the last day's crawl counts, the sitemaps Bing holds, and the addresses with crawl issues, in plain words. It reads the site with the address in `site.toml`, which is what a person verifies in Bing.
- **IndexNow on file updates.** While `khalid serve` runs, each new content catalog is compared with the last. A live post whose `updated:` date changed, and a live post that was not there before, are sent to IndexNow when `[seo.indexnow]` is on. Other edits are not, because the date changes only on a real update (decision D4). An address sent in the last ten minutes, by the door or by a file, is not sent again, so a door publish that also writes the file is announced once.

## Consequences

- Posts updated while the server was stopped are not sent when it starts, because the first catalog has nothing to compare with. A static site has no running server at all. Both are follow-ups: they need a record of what was last announced, kept in `data/`.
- The report is a command, not an admin screen. An admin screen needs a mockup and the founder's yes first.
- A key Bing refuses gives a plain message that names `KHALID_BING_API_KEY` and how to make a new key.
