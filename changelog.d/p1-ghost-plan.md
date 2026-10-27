### Added

- `ghost::plan` works out the content file of every Ghost post and page, read back with `khalid-content` before it is planned. Published posts keep their slugs and dates; drafts, scheduled posts and members-only posts become drafts, so nothing paid is published; newsletters and unpublished or members-only pages stay behind with the reason. Ghost's `__GHOST_URL__` becomes the old site's address (KH-API-7).
