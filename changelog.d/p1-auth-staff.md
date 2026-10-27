### Added

- Owners, admins and editors can list staff (`GET /api/admin/v1/staff/`) and invite someone (`POST /api/admin/v1/staff/`), but only with a role they may manage: an editor invites writers and contributors, and nobody invites a second owner (KH-EDT-7).
