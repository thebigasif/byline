### Added

- `khalid publish <slug>` publishes a draft through the same publish door the Admin API uses: the linter's gate first, then the signed `post.published` webhook and `IndexNow`, or `lint.blocked` when it refuses. `--language` picks a draft in another language, and `--json` answers as the Admin API does (KH-OPS-2, KH-LNT-1, KH-API-3, audit brief G28).
