### Added

- Webhooks are added through the Admin API with `POST /api/admin/v1/webhooks/` and a key that has `webhooks:manage`. The answer shows the secret once, and a webhook added while `khalid serve` runs fires at once (KH-API-2, KH-API-3).
