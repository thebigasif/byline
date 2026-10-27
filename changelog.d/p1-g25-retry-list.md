### Changed

- Failed webhook deliveries are retried to the webhook list as it stands at each round, so a webhook added while `khalid serve` runs is retried and one removed is left alone (KH-API-3).
