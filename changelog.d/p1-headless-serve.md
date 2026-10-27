Added
`khalid serve` answers the sitemaps, `robots.txt`, `llms.txt` and the feeds at the addresses `khalid build` writes them, and sends each post alias on to its post with 301. On a site with `mode = "headless"` it answers those and the Content and Admin APIs, and every page address gets a short 404 that says another app shows the pages. The theme is not read (KH-INS-5, KH-SEO-5 to KH-SEO-8).
