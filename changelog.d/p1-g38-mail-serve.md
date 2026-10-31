### Added

- `khalid serve` sends mail when `site.toml` has a `[mail]` table, so newsletter sign-ups open: the form keeps the address, the confirmation email leaves through SMTP, Resend or Postmark, and its link confirms the subscriber. Without `[mail]` sign-ups stay closed as before. `docs/deploy.md` explains the settings and where the password or key goes.
