### Added

- `PUT /api/admin/v1/authors/{id}/` edits an author: only the fields sent change, an empty role, credentials or photo removes it, and a stale copy is a 409 (KH-API-2, BL-API-7).
