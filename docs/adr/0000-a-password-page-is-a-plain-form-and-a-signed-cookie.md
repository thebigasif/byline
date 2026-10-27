# A password page is a plain form and a signed cookie, and a locked body never leaves the server

**Number:** next free.

**Status:** Proposed, 2026-10-27. Phase 5 task 5.11 and plan section 4.6 (password, access).

## Context

Plan section 4.6 gives a post a password and an access setting, `public` or `members`, with paid tiers later. Task 5.11 asks for gated content and password pages. The public site must read with scripts off (BL-WEB-2), pages are rendered at publish and served from memory (ADR 0024), and content files are the source of truth (BL-FS-1). Readers cannot sign in yet: the accounts track signs in staff only.

## Decision

- **The file keeps a hash.** Front matter holds `password_hash`, an Argon2id hash with its own salt. The Admin API takes the password, hashes it on a blocking thread and never writes, logs or returns it; posts show only `password_protected`.
- **A locked page is rendered at publish.** A post with a password, or for members, gets a public page with its title, its description and a password form or a note. The body is not in the page, its contents list, the feeds, the schema's `wordCount`, the search index or the Content API. A password page is `noindex`.
- **The form is plain HTML.** It posts the post's address and the password to `<mount>_unlock/`. Slugs have no `_`, so no post can take that address. The server checks the password against the hash on a blocking thread.
- **A right password gets a signed cookie and a 303 back to the post.** The cookie is `khalid_unlock_<hash of the address>`, `HttpOnly`, `SameSite=Lax`, `Path` set to the post, `Secure` over HTTPS, for 30 days. Its value is an expiry and an HMAC-SHA256 over the address, the password's hash and the expiry, with the site's preview key and a separate label. So it opens one post, stops working when the password changes, and cannot be made or stretched without the key.
- **A wrong password gets the locked page again with its alert shown,** status 401, never cached. Tries are limited per post, so guessing is slow on top of Argon2id being slow.
- **The full page lives at a hidden address.** `khalid serve` also renders each password post's full page at `/_unlocked<path>`. A request for that address from outside gets 404. A request for the post with a valid cookie is answered from it, `Cache-Control: private, no-store`, so no shared cache keeps it.
- **Members stay locked.** Until readers can sign in, nobody reads a members' post on the public site. A password opens a post even when it is also for members.
- **A static build has no server,** so it writes only the locked page, and static mode replaces the form with a sentence saying password pages need the server.

## Consequences

- A post's body reaches a reader only through the one middleware that checks the cookie, so every new output that reads post files must check `post_lock::Lock` first.
- Changing a post's password signs every reader out of that post.
- Not checked on this server: a real browser's handling of the cookie. The end-to-end test drives the form, the cookie and the page over HTTP.
