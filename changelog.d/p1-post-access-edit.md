### Added

- An Admin API edit can set a post's `access`, `password` (8 to 128 characters, hashed before it is saved, `null` removes it), `template` and `head_code`. Only an owner, an admin or an `admin:all` key may change head code; anyone else gets a 403 that says so, and sending the head code back unchanged is fine. Head code that may not be in a page head is a 422 naming the problem (KH-EDT-3, KH-API-2).
