### Added

- An audit log of every change made through the Admin API, with who made it and when, kept in the site's database and listed at `GET /api/admin/v1/audit/` and on the admin's Audit log screen (KH-EDT-6).
- A trash: a deleted post moves to `trash/` beside `content/` and can be put back where it was or deleted for good, through `/api/admin/v1/trash/` and the admin's Trash screen (KH-EDT-6).
- Tick boxes on the admin's posts table, to set the author of many posts at once or move them to the trash (KH-EDT-6).
- A toolbar at the foot of each blog page for signed-in staff: edit this post, what the linter says about it, and how often it was read in the last 30 days (KH-EDT-6).

### Changed

- Deleting a post through the Admin API moves it to the trash instead of removing its file.
