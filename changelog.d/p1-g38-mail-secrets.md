### Added

- The secret behind `[mail]` comes only from the environment: `KHALID_SMTP_PASSWORD`, `KHALID_RESEND_API_KEY` or `KHALID_POSTMARK_TOKEN`. Khalid refuses to start when the one the chosen way of sending needs is missing, and never prints its value.
