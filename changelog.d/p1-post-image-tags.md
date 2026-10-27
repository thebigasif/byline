### Added

- Post front matter reads `image` (the lead image, with `src`, `alt` and an optional `caption`), `seo_title`, `tags` and `sitemap: false`. A lead image without alt text, a blank tag and a tag named twice are refused with a message that says what to write.

### Changed

- When a file has a field Khalid does not know, the fix suggests the nearest field name, such as "rename it `title`", and lists the allowed fields only when there are 12 or fewer.
