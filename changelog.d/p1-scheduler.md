### Added

- `khalid serve` runs a scheduler: every 5 seconds, each post whose publish time has come goes live, and each whose unpublish time has come is taken down, through the same publish door as the Admin API, so the webhooks and IndexNow are told as usual. After a restart nothing happens twice, and a post the linter refuses stays a draft (KH-API-2, KH-API-3).
