### Added

- `khalid serve` fires `page.unpublished` when a page's file moves from `content/` to the same place under `drafts/`, and the same for glossary terms, case studies and changelog entries. A file removed with no draft left still fires `.deleted` (KH-API-3).
