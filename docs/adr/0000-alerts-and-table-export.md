# 0000. Alerts go to email and one webhook event, and every table exports without its secrets

**Number:** next free. **Status:** Proposed, 2026-10-31. Gap brief G44, KH-GRO-6.

## Context

KH-GRO-6 asks for alerts by email or webhook for broken links, 404 spikes, certificate problems, failed scheduled posts and stale posts, and for CSV and JSON export of every table. The plan says neither who receives an email, how often checks run, what a spike is, nor how an export treats the hashed keys and webhook secrets some tables hold. The founder said yes in advance (2026-10-27), so these are recorded here.

## Decision

- **One webhook event, `alert.raised`.** Each alert is one delivery through the existing dispatcher, signed and retried like every other event, so Zapier, Make or Slack can take it with `khalid webhooks add --event alert.raised`. One event with a `kind` field keeps the catalogue short; receivers filter on `kind`.
- **Email goes to `[alerts] email` in `site.toml`, through `[mail]`.** Without `[mail]` the address is refused at start, with the fix. Without an address, alerts go to webhooks alone.
- **`khalid serve` checks once an hour**, and once a minute after start. Each round looks at:
  - **404 spikes:** the 404 log's total hits against the last round's. More new 404s in the hour than `[alerts] not_found_spike` (50 unless set) raises one alert naming the addresses that grew most. The 404 log is read, never changed.
  - **Certificates:** the certificate `khalid serve --domain` keeps in `data/https/`. Renewal starts 30 days before the end, so a certificate with under 14 days left means renewal is failing. An expired one is its own alert.
  - **Stale posts:** a live post whose last change is older than `[alerts] stale_after_days` (365 unless set, 0 switches it off).
- **Failed schedules are raised as they happen,** by the scheduler, when the linter refuses a post whose publish time came.
- **Broken links are raised by `khalid audit`,** which already finds them. Crawling the whole site every hour from `serve` would cost more than the alert is worth.
- **Each alert is raised once.** Its key, such as `stale:en.old-post`, goes in a new `alerts_raised` table. A condition that clears, such as a renewed certificate, drops its key, so it is raised again if it comes back. Keys older than 30 days are dropped.
- **`khalid db export` writes every table** to `data/export/` as one CSV and one JSON file per table, or to `--out`. Columns that hold a secret or its hash (`secret`, `token_hash`, `key_hash`, `password_hash`, and any later column whose name says hash, secret, token or password) are left out, and a test fails when a new such column is not listed. Tables of only secrets, such as `site_keys`, are left out whole. A CSV field that a spreadsheet would read as a formula gets a leading `'`, as the audit's CSV does.

## Consequences

- An owner without `[mail]` still gets every alert by webhook, and sees each in the server log.
- A restart keeps the raised keys, so it does not send the same alerts again.
- The hourly 404 check needs the previous reading, so a spike that starts in the first hour after a restart is seen from the second round on.
- The export is a snapshot read on one connection; a table written to while it runs may be a moment newer than another.
