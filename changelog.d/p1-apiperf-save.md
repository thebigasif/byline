### Changed

- After a saved post, `khalid serve` makes again only the posts whose files changed, in the Content API and in `data/site.db`, instead of every post of the site, so a save costs the same on a blog of ten posts or ten thousand (KH-API-1, KH-OPS-4).
