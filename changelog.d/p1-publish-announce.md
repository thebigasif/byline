### Added

- `khalid serve` tells the world when a post goes live through the Admin API: it fires the signed `post.published` webhook to every webhook in the site's list that asked for it, sends the post's address to IndexNow when `[seo.indexnow]` is set, and tries failed webhook deliveries again every minute (KH-API-3, KH-SEO-11).
