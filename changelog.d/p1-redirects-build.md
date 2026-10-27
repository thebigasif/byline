Added
`khalid build` writes the site's stored 301, 302 and 307 redirects into `_redirects` after the aliases, each chain in one step, so a static site keeps the redirects it imported. Stored 410, 451 and regex rules stay with `khalid serve`, since static hosts cannot read them, and a build never makes a database (KH-SEO-8).
