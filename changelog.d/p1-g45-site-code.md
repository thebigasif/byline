### Added

- A `[code]` table in `site.toml` holds the site's own code for every page: `head` for the end of the head and `body`, scripts only, for the end of the body, with `consent = "ask"` (the default) or `"not-needed"`. A static site that would ask for consent is refused at start, because a static host cannot keep a reader's choice (KH-SEO-12, G45).
