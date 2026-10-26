Added
`site.toml` takes a `[seo.index]` table that sets each archive type, `tags`, `categories`, `authors`, `dates` and `search`, to `"index"` or `"noindex"`, and `thin_archives_below`, the post count under which a tag or author archive is kept out of search. `khalid build` writes the setting into each archive page's robots tag (KH-SEO-9).
