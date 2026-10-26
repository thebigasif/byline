### Added

- API keys are stored in the site's database, `data/site.db`, as a hash of each key and never the key itself. A revoked key stops working at once, and its identity is never reused (KH-API-2).
