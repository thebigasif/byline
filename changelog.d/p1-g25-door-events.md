### Added

- `khalid serve` fires `post.published.edited` when an edit changes a live post, and tells IndexNow about the change. An edit that changes nothing is not announced (KH-API-3).
- A post the linter refuses to publish fires `lint.blocked` with the post as it stayed and every finding, the same ones the Admin API answers with. Scheduled posts the linter refuses fire it too (KH-API-3, KH-LNT-1).
