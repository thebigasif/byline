### Added

- `khalid serve` looks at the blog's home page at its public address every 5 minutes, as a reader would, and alerts the owner by email and the `alert.raised` webhook when the site stops answering (`site-down`) or answers slowly for an hour (`slow-site`). Set `watch_minutes` and `slow_ms` in `[alerts]`; 0 switches each off.
- `khalid watch` shows how fast and how reliably the blog answered over the last day and the last 30 days, in plain words or as JSON.
