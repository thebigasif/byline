### Added

- An HTML sitemap page at `/sitemap/` in each language links every page search engines may list, grouped as posts, pages, and topics and archives (KH-SEO-5, G12).
- `khalid serve` answers every list page `khalid build` writes: archive, authors, collections, categories, channels, dates and the sitemap.

### Fixed

- List pages pair across languages by keys such as `list:archive`, so a post whose `translation` is `archive` or `sitemap` no longer clashes with them.
