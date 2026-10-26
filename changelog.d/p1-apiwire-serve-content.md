### Added

- `khalid serve` answers the Content API at `/api/content/v1/` with a content key made by `khalid keys create --kind content`. Posts come from the site's index in `data/site.db`, which the server writes again from your files at start and after each saved edit, so an edit shows in the API without a restart (KH-API-1).
