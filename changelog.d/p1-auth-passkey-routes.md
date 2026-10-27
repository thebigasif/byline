### Added

- The Admin API adds a passkey to the signed-in account (`POST /api/admin/v1/session/passkeys/options/`, then `POST /api/admin/v1/session/passkeys/`) and signs in with one (`POST /api/admin/v1/session/passkey/options/`, then `POST /api/admin/v1/session/passkey/`), with no email or password to type (KH-EDT-7, KH-API-2).
