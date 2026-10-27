### Added

- Signed-in staff carry a session cookie that scripts cannot read, that is sent only over HTTPS and only to this site, and that ends after 14 days, or 12 hours unused. Only a hash of it is stored, so a copy of the database signs nobody in (BL-SEC-3, BL-SEC-9).
