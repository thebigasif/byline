# 0000. Newsletter sign-ups use double opt-in with hashed link tokens

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 5 task 5.5.

## Context

Task 5.5 asks for sign-up through a plain form with double opt-in, subscribers stored as visitor data, a confirmation email, and one-click unsubscribe, tested with a fake mail transport. The Phase 5 answers (`docs/decisions/2026-10-27-phase-5-answers.md`) settle that the newsletter track adds a forward migration, that mail settings and SMTP come with task 5.6, and that the `::newsletter` block gains `list=` in task 5.7. The plan says every public form works with scripts blocked. Nothing yet says what a link does when it is opened, how long it works, or what the site shows before it can send email.

## Decision

- **The logic lives in `khalid-newsletter`, the rows in `khalid-db`.** `SubscriberStore` is the line between them, as `KeyStore` is for API keys (ADR 0011). `khalid-newsletter` also holds the `Mailer` trait, the pages and the routes; `crates/khalid` only builds the router and merges it.
- **Tokens are 256 random bits stored as their SHA-256 hash** (BL-SEC-3). A random token needs no slow hash, and looking a link up by its hash leaks nothing about other links. Each email gets its own unsubscribe link, so no stored value can rebuild one.
- **Opening a link changes nothing.** The confirmation and unsubscribe pages show one button, and only its post acts. Mail programs and virus scanners open links on their own, and a confirmation they trigger would not be the reader's yes. A mail program's one-click post to the unsubscribe link (RFC 8058) unsubscribes at once.
- **A confirmation link works once and for two days.** A second confirmation email to the same address waits ten minutes, so a form sent over and over cannot fill an inbox. The confirmation email also carries a link that removes the address at once.
- **The reader sees the same page after every sign-up,** whether the address was new, waiting or confirmed, so the form never reveals who subscribes.
- **Unsubscribing leaves every newsletter of the site.** Leaving one newsletter can come with task 5.7.
- **The form answers at `subscribe` under the mount path and at `/subscribe`,** where the `::newsletter` block's form posts today.
- **Until a site can send email, every newsletter page says sign-ups are not open, and nothing is stored.** `khalid serve` passes no transport until task 5.6 reads the mail settings. Storing addresses that can never be confirmed would keep personal data for no purpose.
- **The pages are written by this crate, not the theme.** They stand alone in the site's default language, so they work in any mode and before the first render.

## Consequences

- On a running site the form answers 503 with "Signing up is not open yet" until task 5.6 lands. The whole flow is proven over HTTP against SQLite and the fake transport in `crates/khalid-newsletter/tests/routes.rs`.
- Every email sent with task 5.8 adds one unsubscribe row per reader. That is one small row per email and person, cheap next to the email itself.
- The pages do not wear the site's theme yet. When the render track offers a page shell, they can move into it.
- A site mounted under a folder of another site, behind its proxy, receives the block's form only if the block posts under the mount path. That is a change for the content track.
