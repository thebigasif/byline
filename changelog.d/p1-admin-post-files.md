### Added

- `khalid serve` keeps a post's settings in its file: an Admin API edit writes the date, excerpt (as `description`), featured flag, robots, canonical, aliases and author into the front matter, leaves every other line as it was, and reads them back. A new slug moves a draft to its new file. A setting the files cannot hold yet (co-authors, feature image, review state, share cards), a new slug for a live post, or removing a live post's date is a 422 that names it, and nothing is changed (KH-EDT-3, KH-API-2).
