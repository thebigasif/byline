### Changed

- `khalid serve` builds the Content API and Admin API when the first request reaches them instead of at start, and serves the OpenAPI document as text made once. An idle server on a 1,000-post site holds about 9.5 MB less memory, back under its 55 MB budget, and `/api/openapi.json` answers in a third of a millisecond.
