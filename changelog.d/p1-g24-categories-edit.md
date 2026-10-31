### Added

- `PUT /api/admin/v1/categories/{slug}/` replaces a category's names, and `DELETE` removes a category no post is in; a stale copy, or posts still in the category, is a 409 (KH-API-2, BL-API-7).
