# CTO answers, 2026-10-27

Questions raised by the performance track while measuring task 1.21.

## 1. `notify` for file events

Yes. `khalid serve` may use `notify` to see saved files at once instead of looking every half second. Its licence, CC0-1.0, is public domain and allowed for this crate only in `deny.toml`. Polling stays as the fallback where file events are not available, such as some network drives.

## 2. What "idle memory" means

Idle memory is the resident set size of `khalid serve` measured 15 seconds after it answered its first request, with no traffic in between, on the 1,000-post site. The allocator may be tuned to hand freed memory back, for example mimalloc's `no_thp` and purge settings. Record any setting in `docs/standards/performance.md` with the measurement that justifies it. The 55 MB budget does not change.

## 3. Start-up and save time after the Content API landed

The Content API must not delay the first answer or a save. `khalid serve` answers pages before the API's store is ready, builds that store from the index without loading every post body, and on a save updates only the changed post. The API track owns this; the budgets in `docs/standards/performance.md` do not change.

## 4. Scheduling and preview links (task 5.1)

- Publishing has one door, the Admin API's publish operation. It fires the `post.published` webhook from the site's webhook list and sends the changed addresses to IndexNow when that is switched on. The scheduler calls the same operation at the post's time; it never publishes on its own.
- An Admin API edit accepts `published_at`. A future time with status `scheduled` schedules the post; `unpublish_at` schedules its removal.
- A preview link works for 7 days by default. A site may set it from 1 hour to 30 days with `preview_link_days` or `preview_link_hours` in `site.toml`. Links are signed, expire, and pages behind them carry `noindex`.
- The editor mockups gain a publish-time control and a "copy preview link" button in post settings. The founder sees them with the rest of the mockups.
