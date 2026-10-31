### Added

- The site audit reads what a search engine reads first: `robots.txt` at the root of the host, the sitemaps it names inside the blog or the blog's own `sitemap.xml`, and the blog's `llms.txt`. It visits the pages only a sitemap lists after the pages links reach, with no click depth (KH-LNT-5).
