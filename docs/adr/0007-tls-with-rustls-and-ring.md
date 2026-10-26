# 0007. TLS through rustls with the ring provider

**Status:** Accepted, 2026-10-26. A consequence of ADR 0005, made in Phase 1 task 1.4.

## Context

ADR 0005 allows permissive licences only and bans OpenSSL, so the binary has one TLS stack and nothing SerpDad could not ship. The outbound client uses `reqwest`. Its default TLS in version 0.13 is rustls with the aws-lc-rs crypto provider, and aws-lc carries OpenSSL licence terms that our `cargo deny` allow-list refuses. It also builds a large C library.

## Decision

`reqwest` runs with `rustls-no-provider`, and `khalid-outbound` installs rustls's `ring` provider when it builds the client. `ring` is licensed Apache-2.0 and ISC, both on the allow-list.

## Consequences

- One TLS stack, rustls, with a permissive crypto library, and no OpenSSL anywhere in the tree.
- Any crate that adds TLS later uses the same provider, not a second one.
- If `ring` falls behind aws-lc on algorithms we need, this record is superseded with a reason, not edited.
