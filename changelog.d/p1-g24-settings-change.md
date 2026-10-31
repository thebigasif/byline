### Added

- `PUT /api/admin/v1/settings/` changes the site's name, address, mount or languages: only the fields sent change, a value the site could not start with is refused naming the field, and a stale copy is a 409 (KH-API-2, BL-API-7).
