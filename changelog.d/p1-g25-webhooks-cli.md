### Added

- `khalid webhooks add --event <event> --url <address>` adds a webhook and shows its secret once, `khalid webhooks list` shows the webhooks that are on, and `khalid webhooks remove <id>` switches one off. Each takes `--json`, and a running `khalid serve` sees the change at once (KH-API-3, KH-OPS-2).
