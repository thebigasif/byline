### Added

- `site.toml` takes a `[mail]` table: the address mail comes from, an optional sender's name, and exactly one way to send it, `[mail.smtp]`, `[mail.resend]` or `[mail.postmark]`. Unencrypted SMTP is refused unless `private_host = true` says the server is on the site's own network.
