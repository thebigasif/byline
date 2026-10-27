Added
`khalid doctor` checks the host's own `robots.txt` for a blog in a folder: it reports a problem when the file is missing or does not name the blog's sitemap, and says which line to add or which file to copy. `--offline` skips it, and a blog at the root of its host has nothing to check (KH-SEO-5, KH-OPS-2, audit G3).
