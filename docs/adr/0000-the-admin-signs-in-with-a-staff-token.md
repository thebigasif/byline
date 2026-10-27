# 0000. The admin signs in with a staff token

**Number:** next free.
**Status:** Proposed, 2026-10-27. Phase 4, the admin app, part 1.

## Context

The admin in `admin/` is a React app that talks only to the Admin and Content APIs (plan section 4.6, ADR 0003). Accounts with email, password and passkeys come later in Phase 4. Today the Admin API takes an integration key or a staff token in `Authorization: Khalid <key>` (ADR 0011), and it has no route that says whether a key works.

## Decision

- **Sign-in takes a staff token** made with `khalid keys create --kind staff`. The field is a password field, so the token is never shown on screen.
- **The token lives in the tab's `sessionStorage`** under `khalid-admin-key`. It survives a reload, ends when the tab closes, and is sent only to the blog that served the admin, on the same origin. Signing out removes it. It is never logged or put in an address.
- **The check reads a post that cannot exist**, `GET /api/admin/v1/posts/khalid-admin-sign-in-check/`. The key layer answers 401 for a refused key and 403 for a key without `posts:write` before the route looks for the post, so a 404 means the key works. When the API gains a route that names the signed-in person, the check moves onto it.
- **API types are generated, not committed.** `npm ci` runs `openapi-typescript` over `sdk/typescript/openapi.json` into `admin/src/api/schema.d.ts`, which git ignores, and `openapi-fetch` sends typed requests from it. A changed route changes the admin's types on the next install, and `tsc` fails where the admin no longer fits.

## Consequences

- A token in session storage can be read by script running on the admin's page. The admin loads no third-party script, and accounts with an HTTP-only session cookie replace this in Phase 4.
- Every request carries the token, so the server checks the key on each one, as it does for any API client.
- The probe costs one Argon2id check at sign-in, tens of milliseconds.
