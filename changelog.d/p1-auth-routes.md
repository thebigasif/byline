### Added

- `POST /api/admin/v1/session/` signs staff in with their email and password and sets the session cookie; `GET` on the same address says who is signed in. Too many failed tries get a 429 with `Retry-After` (KH-EDT-7, KH-API-2).
