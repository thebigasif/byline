### Added

- `khalid serve` fires `page.published`, `page.published.edited` and `page.deleted` as a page's file is added, changed or removed, and the same three events for glossary terms, case studies and changelog entries. The body carries the entry's id, language, slug, title and file (KH-API-3).
