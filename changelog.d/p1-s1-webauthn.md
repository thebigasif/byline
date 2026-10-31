### Changed

- Passkeys are checked by `webauthn-rs`, a widely used library, instead of Khalid's own code. It no longer needs OpenSSL, so Khalid stays on one TLS and crypto stack.
