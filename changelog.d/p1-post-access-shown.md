### Added

- The Admin API sends four more post settings: `access` (`public` or `members`), `password_protected` (whether readers need a password; the password and its hash are never sent), `template` and `head_code`. A post password is kept only as an Argon2id hash with its own salt (KH-EDT-3, KH-API-2).
