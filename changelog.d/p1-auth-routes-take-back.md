### Added

- Staff can sign out (`DELETE /api/admin/v1/session/`), sign out on every device at once (`DELETE /api/admin/v1/sessions/`), and change their password (`PUT /api/admin/v1/session/password/`), which signs out every other device (KH-EDT-7).
