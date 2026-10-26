Added
`khalid build` writes `_redirects` when posts list `aliases`: one `from to 301` line per old address, which Netlify and Cloudflare Pages follow as they are. An alias outside the blog's folder, or one claimed by two posts, stops the build with a plain message (KH-SEO-8).
