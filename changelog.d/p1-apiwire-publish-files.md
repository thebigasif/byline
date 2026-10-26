### Added

- Posts made through the Admin API are drafts in `drafts/<language>/`, which readers never see. Publishing one runs the linter first. A post it blocks stays a draft, and a post it lets through moves into `content/` with today's date. Edits change only the fields they name and keep every other line of the file (KH-API-2, KH-LNT-3).
