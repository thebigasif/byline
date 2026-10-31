### Added

- A visitor log for `khalid serve`, off until `[visitor_log] on = true` in `site.toml`. It keeps one row per visit for 7 to 14 days with a shortened address, a visitor code that changes every day, and whether a bot's address is on its company's published list. No cookies. The privacy design is in `docs/design/visitor-log.md`.
