Added
`site.toml` takes a `[seo.indexnow]` table that switches IndexNow on: `key`, 8 to 128 letters, digits and hyphens, and an optional `endpoint`, which defaults to the shared `https://api.indexnow.org/indexnow`. IndexNow tells Bing and the other search engines that share it about a changed page at once. Nothing is sent yet; publish sends it once the publish path lands (KH-SEO-11).
