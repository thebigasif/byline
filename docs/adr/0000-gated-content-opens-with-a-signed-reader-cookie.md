# Gated content opens with a signed reader cookie after the sign-up is confirmed

**Number:** next free.

**Status:** Proposed, 2026-10-31. Phase 5 task 5.11, gap brief G41, plan section 5 ("gated content that unlocks with an email address").

## Context

The `:::gated` block shows a note and a sign-up form and never its body (KH-GRO-3). Password pages are done (ADR "a password page is a plain form and a signed cookie"). Mail leaves through `khalid-mail` (ADR "mail leaves through khalid-mail"), and sign-ups are double opt-in (ADR "newsletter double opt-in"). The public site must read with scripts off (BL-WEB-2), pages are rendered at publish and served from memory, and readers have no accounts.

## Decision

- **Confirming is the unlock.** A reader reads gated content once they confirm their address, the same step that makes them a subscriber. Typing an address alone opens nothing, so no one reads by typing someone else's address.
- **The form names the page.** The gated block's form carries `back`, the post's address. It goes into the confirmation link and then beside the confirm button. Only a path under the mount with no `//`, backslash, space, quote or bracket is kept, so a link can never send a reader to another site.
- **One cookie for the whole blog.** Confirming answers `303` to `back` with `khalid_reader`: an expiry and an HMAC-SHA256 over a label and the expiry, signed with the site's preview key, `Path` the mount, `HttpOnly`, `SameSite=Lax`, `Secure` over HTTPS, for 180 days. It holds no address and no subscriber number, so it says nothing about who reads, and opens every gated post. The label differs from the password cookie's and the preview link's, so none passes for another.
- **A new device gets a pass link.** A confirmed address that signs up again from gated content gets one email with `subscribe/pass?code=...&back=...`, good for two days, signed under its own label. Opening it sets the cookie and goes back to the post. It stores nothing, so a mail program that scans links does no harm, and the form still answers everyone the same way.
- **The full page is the public page with the parts swapped.** `khalid serve` keeps, for each post with gated content, a copy of its finished public page with each locked part replaced in turn by the part rendered open. The copy lives at the hidden `/_unlocked<path>` beside password pages and is answered only through the cookie check, `private, no-store`. The locked page says `Vary: cookie`.
- **Password and members posts win.** Their whole body is locked, so their gated parts never open on their own.

## Consequences

- Unsubscribing does not take the cookie back; it runs out after 180 days. Checking the subscriber on every request would need a database read per gated page.
- The table of contents and the feeds come from the public page, so a heading inside a gated part is not in either. The search index and the Content API never see the gated body.
- Images inside a gated part get no generated sizes, because sizes are made from the public page.
- A static build has no server, so gated content stays locked there, as the static lock card says.
- Not checked on this server: a real browser's handling of the cookie and the redirect. The end-to-end test drives the form, the email, the button, the cookie and the page over HTTP.
