# 0000. Staff accounts live in khalid-auth, with server-side sessions and passkeys checked on ring

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 4, the accounts track (plan section 4.6, PRD KH-EDT-7, RULES.md BL-SEC-3, BL-SEC-5, BL-SEC-9).

## Context

The admin needs staff to sign in with an email and password, a passkey or a magic link, and the five roles need a permission table. Until now the admin signs in with a staff token (ADR "the admin signs in with a staff token"), and the Admin API checks integration keys and staff tokens (ADR 0011). Migration 0003 already holds `users` and `sessions` tables. The brief names `webauthn-rs` for passkeys, but every release of it verifies signatures through OpenSSL, which `deny.toml` bans (ADR 0005, ADR 0007).

## Decision

- **One crate, `khalid-auth`.** It owns the permission table, accounts, passwords, passkeys, magic links and sessions, behind store traits. `khalid-db` stores them on SQLite with a forward migration, the way it stores keys for `khalid-api`. The roles stay `khalid_api::keys::Role`, so a staff token and a signed-in account with the same role may do the same; a test pins that.
- **Roles.** The table is in the crate's `README.md`, checked by a test. Nobody manages the owner; the site changes hands only by a transfer. A role change needs the right to manage the old and the new role.
- **Passwords** hash with Argon2id on a blocking thread. An unknown email is checked against a fixed dummy hash, so a wrong email and a wrong password take the same time and get the same answer.
- **Sessions** are 256 random bits in a cookie named `__Host-khalid-session`, `HttpOnly`, `Secure`, `SameSite=Lax`, `Path=/`. Only the SHA-256 of the token is stored, so a copy of the database signs nobody in. A session ends after 14 days, or after 12 hours unused. Changing a password or signing out everywhere ends every other session.
- **CSRF.** Every request that uses the session cookie must also send the `X-Khalid-CSRF` header with the session's CSRF token, which the sign-in answer returns. The token is derived from the session secret, so it needs no storage and is useless without the cookie. A form on another site can neither read it nor set a custom header.
- **Passkeys** follow WebAuthn Level 2 with ES256 and EdDSA keys. Khalid asks for no attestation, so it checks the challenge, the origin, the relying party, the user-presence and user-verification flags and the signature counter, and verifies signatures with `ring`, which the tree already uses. CBOR is read with `ciborium`. Tests register and sign in with a software authenticator built on `ring`.
- **Magic links** are single-use tokens, stored hashed, valid for 15 minutes. They are sent through a `LinkSender` trait; the newsletter track's mail sender implements it when it lands.
- **Sign-in is rate limited** per email and per address: after 5 failures in 15 minutes, more attempts are refused with a plain message until the window passes.

## Consequences

- No OpenSSL. Two new dependencies at most, `ring` (already in the tree) and `ciborium`.
- Passkey verification is our own code, about 300 lines, so it carries its own tests for each refusal: a wrong origin, a wrong challenge, a replayed counter and a bad signature.
- RSA passkeys (RS256) are not accepted. Every current platform authenticator offers ES256, so this is rare; adding RS256 is a later change in one module.
- Staff tokens keep working beside sessions, so the admin and agents that sign in with a token are unaffected.
