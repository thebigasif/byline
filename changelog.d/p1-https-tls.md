### Added

- A TLS listener answers each connection over HTTPS with the site's current certificate, takes a renewed certificate from the next connection without a restart, and closes a handshake that stalls for ten seconds without holding up other visitors (KH-INS-4).
