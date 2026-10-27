### Added

- Every language of a blog has a search page at `search/`, such as `/blog/search/` and `/blog/de/search/`. In server mode it is a plain form that works with scripts off, every page's header links to it, and the site tells search engines about it so they may show a search box. In static mode it shows a lock card that says search needs the server and how to turn it on. Search pages ask search engines not to list them, unless `[seo.index] search` says otherwise (KH-WEB-2, KH-OPS-1).
