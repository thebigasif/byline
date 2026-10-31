# 0000. Passkeys are checked by webauthn-rs, on RustCrypto

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase GEO task S1 (`docs/phases/phase-geo.md`), PRD KH-EDT-7, RULES.md BL-SEC-3, BL-DEP-2, BL-SEC-8. Replaces the passkey part of ADR "staff accounts live in khalid-auth".

## Context

ADR "staff accounts live in khalid-auth" checked passkeys with our own WebAuthn code, about 570 lines on `ring` and `ciborium`, because every `webauthn-rs` release then verified signatures through OpenSSL, which `deny.toml` bans. Task S1 asks for `webauthn-rs` instead, so the hardest security code in Khalid is not ours to keep.

The 0.6 line of `webauthn-rs` (0.6.1-dev on crates.io, the newest release) moved off OpenSSL to `crypto-glue`, a thin wrapper over the RustCrypto crates. `cargo tree -i openssl-sys` finds nothing. Two things remain:

- Its crates, and `crypto-glue`, are MPL-2.0. BL-DEP-2 allows MPL-2.0 by named exception, as it already does for Servo's CSS parser.
- `cargo deny` reports two timing advisories in the RustCrypto releases it uses: RUSTSEC-2023-0071 (`rsa`, the Marvin attack) and RUSTSEC-2026-0387 (`crypto-bigint` 0.5). Both leak through work on secret keys. A relying party only verifies signatures with public keys.

## Decision

- **`webauthn-rs` 0.6.1-dev** with default features off (no attestation) and `conditional-ui` on, which gives the discoverable sign-in the admin uses: the person picks a passkey before Khalid knows who they are.
- **Named exceptions** in `deny.toml` for `webauthn-rs`, `webauthn-rs-core`, `webauthn-rs-proto`, `webauthn-attestation-ca` and `crypto-glue`, and the two advisories ignored with the reason above and a review date of 2027-04-26.
- **Khalid keeps its own edges.** `khalid-auth` builds the `Webauthn` value from the site address, keeps each waiting ceremony in memory for five minutes, and stores each passkey as the JSON `webauthn-rs` serialises, so nothing in Khalid reads CBOR, raw public keys or authenticator data any more.
- **Sign-in looks the passkey up by its credential identity**, then lets `webauthn-rs` check it. A counter that goes back is refused, so a copied passkey signs nobody in.
- **The user handle** is a UUID made from the account number, so it is stable for an account and says nothing else about it.
- **Stored passkeys start again.** Migration 0018 rebuilds `passkeys` with a `credential` column for the JSON. The old rows held our own short key form, which `webauthn-rs` cannot read. Khalid has no release yet, so no real site holds a passkey; staff on a test site add theirs again.
- **Tests** register and sign in with `webauthn-authenticator-rs`'s software passkey, a test-only dependency, so the passkey on the other side is not our code either.
- **Passwords stay on Argon2. Sessions stay as they are.** `tower-sessions` is optional in S1 and is not taken: the sessions code has its own tests and a CSRF design that `tower-sessions` does not give.

## Consequences

- About 570 lines of our own WebAuthn code go, with `ciborium` and `ring` in `khalid-auth`.
- The passkey routes now speak the standard WebAuthn JSON: the options answer is what the browser's `PublicKeyCredential.parseCreationOptionsFromJSON` and `parseRequestOptionsFromJSON` read, and the bodies carry the credential's `toJSON()` with the challenge it answers beside it. No admin screen called the old shapes yet.
- The accepted keys are the ones `webauthn-rs` calls secure: ES256, ES384 and RS256. Ed25519 passkeys, which the old code took, are no longer offered; phones, laptops and security keys all make ES256 keys, so a person meets this only with a rare key that makes nothing else.
- Khalid depends on a `-dev` release. Cargo picks 0.6.1 when it is published; a later minor release is a reviewed upgrade like any other.
- Error messages are coarser: `webauthn-rs` names the failed check in its error, and Khalid groups those into the few plain reasons the admin shows.
