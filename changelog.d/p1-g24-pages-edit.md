### Added

- `GET` and `PUT /api/admin/v1/pages/{id}/` read and edit one page, such as `en.about`. An edit changes only the fields it sends and names the `updated_at` of the copy it was made on; a stale one is a 409 and changes nothing (KH-API-2, BL-API-7).
