### Added

- `khalid serve` raises alerts: a burst of 404s, an HTTPS certificate near its end, stale posts each hour, and a scheduled post the linter refused as it happens. They go to `alert.raised` webhooks and, with `[alerts] email`, by email.
