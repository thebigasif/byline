Added
`khalid build` on a site with `mode = "headless"` writes the sitemaps, `robots.txt`, `llms.txt`, the feeds and `_redirects`, and no HTML and no stylesheet. The theme is not read, so a broken one does not stop the build (KH-INS-5).
