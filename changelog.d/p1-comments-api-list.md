### Added

- The Admin API lists comments newest first, filtered by status, at `GET /api/admin/v1/comments/?status=pending`, and approves one with `POST /api/admin/v1/comments/{id}/approve/`. Both need a key with `posts:write`.
