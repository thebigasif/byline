### Added

- `GET /api/admin/v1/search-console/follow-ups/` lists the long searches that showed each post in the last 28 days, with the heading that answers each. Needs `settings:write` (GEO M3). A site with no Search Console connection answers 409 `not-set-up`; Google refusing answers 502 `upstream`.
