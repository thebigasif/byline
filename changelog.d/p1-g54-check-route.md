### Added

- `GET /api/admin/v1/posts/{id}/check/` checks a post as it would go live now and saves nothing: every lint finding with the score out of 100, the search result its page would make (title, address and description), and the JSON-LD the page would carry. The editor's SEO panel reads it (KH-SEO-1, KH-SEO-4, KH-LNT-4).
