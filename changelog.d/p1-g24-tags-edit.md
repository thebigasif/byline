### Added

- `PUT /api/admin/v1/tags/{id}/` renames a tag in every post that has it, merging it into a tag of the same name, and `DELETE` removes it from them all; a post changed since the copy the request was made on stops the change with a 409 (KH-API-2, BL-API-7).
