### Added

- Rate limits on the Content API and the Admin API: 600 Admin API requests a minute per key, 6,000 Content API requests a minute per content key, and 10 wrong keys a minute per address. Past a limit the API answers 429 with `Retry-After` and says in plain words how long to wait.
