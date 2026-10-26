# 0014. Webhook event names and signatures

**Status:** Accepted, 2026-10-26. Phase 1 task 1.19.

## Context

`docs/standards/api.md` fixes the header, `Khalid-Signature: t=<unix>,v1=<hmac-sha256(secret, t + "." + body)>`, and lists the events, with "the same for `page` and each collection". It leaves four things open: what the collection events are called, how long a receiver should accept a signature, how long a signing key must be, and where webhooks live in the workspace.

## Decision

- **Collection event names** use the resource in the singular with hyphens: `glossary-term.published`, `case-study.published`, `changelog-entry.published`, and the same five verbs as posts. Hyphens keep each name in the `resource.verb` shape with one dot before the verb.
- **The receiver's window** is five minutes either side of its clock. A delivery signed earlier or later is refused as a possible replay. The `Verifier` can widen it.
- **Signing keys** are at least 32 bytes, the 256 bits RULES.md BL-SEC-3 asks of tokens. The HMAC key is the UTF-8 bytes of the secret as the receiver sees it, so any HMAC library can check a delivery.
- **Several `v1` values** may appear in one header, and any match passes, so a sender can rotate keys without a gap. Unknown keys such as a future `v2` are skipped.
- **The crate** is `khalid-webhooks`, apart from the server, so the CLI and tests can sign and verify without pulling in HTTP routing.

## Consequences

- A receiver written with Stripe's or Ghost's signature code needs only the header name changed.
- Adding an event is additive and stays in `v1`. Renaming one is a `v2` change.
