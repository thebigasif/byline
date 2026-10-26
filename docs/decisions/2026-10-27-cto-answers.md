# CTO answers, 2026-10-27

Questions raised by the performance track while measuring task 1.21.

## 1. `notify` for file events

Yes. `khalid serve` may use `notify` to see saved files at once instead of looking every half second. Its licence, CC0-1.0, is public domain and allowed for this crate only in `deny.toml`. Polling stays as the fallback where file events are not available, such as some network drives.

## 2. What "idle memory" means

Idle memory is the resident set size of `khalid serve` measured 15 seconds after it answered its first request, with no traffic in between, on the 1,000-post site. The allocator may be tuned to hand freed memory back, for example mimalloc's `no_thp` and purge settings. Record any setting in `docs/standards/performance.md` with the measurement that justifies it. The 55 MB budget does not change.

## 3. Start-up and save time after the Content API landed

The Content API must not delay the first answer or a save. `khalid serve` answers pages before the API's store is ready, builds that store from the index without loading every post body, and on a save updates only the changed post. The API track owns this; the budgets in `docs/standards/performance.md` do not change.
