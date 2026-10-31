### Added

- The Admin API reads and changes three more post settings: the title search results show (`seo_title`), the focus keyword, and hand-written JSON-LD (`schema`). A search title with a broken `{variable}`, or JSON-LD with a slip, is refused before it is saved, and a slip in the JSON names its line and column. `khalid serve` keeps all three in the post's front matter (KH-SEO-1, KH-SEO-4, KH-LNT-4).
