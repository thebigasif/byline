### Added

- `DELETE /api/admin/v1/posts/{id}/?updated_at=...` removes a post in any status. A delete made on an older copy gets a 409, and a live or scheduled post needs `posts:publish` (KH-API-2, BL-API-7).
