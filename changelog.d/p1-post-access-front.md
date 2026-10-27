### Added

- Post front matter takes `access` (`public` or `members`), `template` (such as `wide`, for `theme/templates/post-wide.html`), `password_hash` (the Argon2id hash the Admin API writes, never a password) and `head_code` (checked HTML for the page head). A wrong value is refused with the file, line and field (KH-EDT-3).
