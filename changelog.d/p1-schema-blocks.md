### Added

- A post's FAQ, how-to, review, recipe and video blocks now write their schema (`FAQPage`, `HowTo`, `Review`, `Recipe`, `VideoObject`) into the page's JSON-LD, in build and serve. Videos are also passed on for the video sitemap, and links to Wikipedia or Wikidata articles become the page's `mentions`. A review with no author of its own is credited to the post's author. The demo post "Fixing slow pages" ends with an FAQ.
