### Added

- `POST /api/admin/v1/session/link/` mails a sign-in link to staff, and `POST /api/admin/v1/session/link/redeem/` signs in with the token from it, once. A site that cannot send mail says so in plain words (KH-EDT-7, KH-API-2).
