Fixed
With `[seo.indexnow]` set, `khalid build` writes the `IndexNow` key file, `<key>.txt`, into the blog's folder and `khalid serve` answers it at the same address, the one every request names in `keyLocation`. Before, no file was there, so search engines refused every ping (KH-SEO-11, audit G3).
