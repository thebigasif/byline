### Added

- A post's lead image (`image` in its front matter) is drawn at the top of its page in every size, fetched at once with `fetchpriority="high"`, with its caption, and shared as the page's Open Graph and X image. It works for an image file beside the post; the demo post "Fixing slow pages" shows one.
- A post's `seo_title` becomes its title in search results and the browser tab, and may use the meta template variables such as `{site}`. A post with `sitemap: false` is left out of the XML sitemap.
