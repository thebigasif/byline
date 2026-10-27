### Added

- The Admin API answers a comment as staff with `POST /api/admin/v1/comments/{id}/replies/`, which shows the reply at once and approves the comment, and deletes a comment with its replies with `DELETE /api/admin/v1/comments/{id}/`.
