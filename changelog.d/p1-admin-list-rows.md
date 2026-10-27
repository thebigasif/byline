### Added

- The Admin API has a row for the admin's posts table: a post's title, status, review state, featured flag, dates, lint counts (blocking findings and warnings) and index status (`indexable`, `noindex`, `canonical_elsewhere`, `not_live`). The shared filter grammar reaches every column, such as `filter=lint.blocks:>0+status:draft` (KH-API-2).
