### Added

- Earlier versions of a post (G50, KH-EDT-2): each save that changes a post's title or text keeps the ones it replaced, the 100 newest per post, in the site's database. `GET /api/admin/v1/posts/{id}/versions/` lists them, and the editor's "Earlier versions" shows who saved each and when, what changed since, and brings one back with an ordinary save that can itself be undone.
