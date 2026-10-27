### Changed

- `GET /api/admin/v1/posts/` answers `limit=all` with a 400, because it counts each post's lint findings and a page holds at most 100, and answers `include`, `fields` or `formats` with a 400 naming the four parameters it takes (KH-API-2).
