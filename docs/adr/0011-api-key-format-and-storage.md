# 0011. API key format and storage

**Status:** Accepted, 2026-10-26. Phase 1 tasks 1.17 and 1.18.

## Context

RULES.md BL-SEC-3 says tokens are 256 random bits, stored hashed and compared in constant time. The task brief asks for integration keys and staff tokens hashed with Argon2id. Content keys are different: they sit in web pages by design (`docs/standards/api.md`), and the Content API checks one on every request, which must stay inside the budget of 10,000 cached requests per second. Argon2id is slow on purpose, tens of milliseconds per check.

## Decision

- **One text format** for every key: a kind prefix, a public identity and a secret, joined by `_`. `kc_` is a content key, and task 1.18 adds the prefixes for integration keys and staff tokens. The identity is 16 lowercase hex digits (64 bits) used to look the key up. The secret is 64 lowercase hex digits (256 bits).
- **Content keys** store the SHA-256 of the secret and compare it in constant time. A 256-bit random secret cannot be guessed or reversed from its SHA-256, so a slow hash adds nothing but cost.
- **Integration keys and staff tokens** store an Argon2id hash, as the brief asks, because they can write.
- **One answer for every refusal.** A malformed key, an unknown identity, a wrong secret and the wrong kind all get "the key was not accepted", so a refusal teaches a guesser nothing.
- The secret only ever travels inside `Secret<T>`, so no `Debug` or log line can show it (BL-RUN-5).

## Consequences

- A leaked database does not leak usable keys of either kind.
- The prefix lets secret scanners, and people, recognise a Khalid key and what it can do.
- Changing the hash of an existing kind later means rehashing on next use; the stored record says which hash it holds.
