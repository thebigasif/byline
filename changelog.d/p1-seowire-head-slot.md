Changed
Templates print a page's SEO tags and schema markup from `meta.seo_head` when the build provides them, in place of the base layout's smaller set of tags. A theme that replaces `base.html` prints `{{ meta.seo_head }}` to keep them.
