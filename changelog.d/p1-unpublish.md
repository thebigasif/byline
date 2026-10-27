### Added

- An Admin API edit with `status: draft` takes a live post down: its file moves back into `drafts/`, the `post.unpublished` webhook fires, and IndexNow is told its address changed (KH-API-2, KH-API-3, KH-SEO-11).
