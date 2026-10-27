### Added

- An Admin API edit takes a `settings` object with any of a post's settings: date, co-authors, excerpt, feature image, featured, review state, canonical, robots, share cards and aliases. A setting left out keeps its value, `null` clears one, and a setting that cannot be right is a 422 that says which (KH-EDT-3, KH-API-2).
