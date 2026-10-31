### Added

- `khalid serve` answers `GET /api/admin/v1/search-console/follow-ups/` with the finder `khalid search-console follow-ups` runs, reading the posts on each request (GEO M3). A site that did not opt in is told to set up Search Console and restart the server.

### Changed

- Search Console failures now end with "try again" rather than "run the command again", since the admin shows them too.
