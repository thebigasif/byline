### Added

- `khalid serve` answers the Admin API at `/api/admin/v1/` with an integration key or staff token made by `khalid keys create`. A post created there is a draft. Publishing it runs the linter, and once it passes, the new page is served within a second, with no restart (KH-API-2, KH-LNT-3).
