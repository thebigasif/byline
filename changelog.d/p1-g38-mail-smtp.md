### Added

- `khalid-mail` sends through an SMTP server with `lettre`: STARTTLS on port 587 by default, TLS from the first byte, or no encryption for a relay marked `private_host = true`. A public host is looked up through the private-network guard and the certificate is still checked against its name. STARTTLS never falls back to plain text.
