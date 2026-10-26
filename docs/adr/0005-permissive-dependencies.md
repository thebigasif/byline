# 0005. Permissive dependencies only, so SerpDad can reuse the crates

**Status:** Accepted, 2026-10-26. From the founder's plan, section 3 and decision 9.

## Context

Byline is AGPL-3.0. SerpDad, a closed hosted product, will depend on crates from this repository, at least `byline-lint`. SerpDad owns the copyright to its own code and the CLA covers outside contributions, so the AGPL on our code is not the problem. A copyleft third-party dependency would be: it would bind SerpDad's backend too.

## Decision

Dependencies carry permissive licences only: MIT, Apache-2.0, BSD, ISC, Zlib, Unicode and the like, enforced by `cargo deny` (BL-DEP-2). MPL-2.0 is allowed per crate by a named exception with a reason. GPL-family dependencies are refused. OpenSSL is banned in favour of rustls, so the binary has one TLS stack and no system library.

## Consequences

- A new dependency with an unusual licence stops CI until it gets an exception or a replacement.
- Crates that SerpDad reuses must not depend on anything SerpDad could not ship.
