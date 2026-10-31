# Consent is a form and a cookie, and only a reader who answered gets a page of their own

**Number:** next free.

**Status:** Proposed, 2026-10-31. G45, KH-SEO-12.

## Context

KH-SEO-12 asks for head and body code per site and per post, allowed through the Content Security Policy, with a consent banner that holds scripts until a reader agrees. The banner must work with scripts off, as every public page must (BL-WEB-2). Pages are rendered once, at publish, and kept with a tag and a Brotli copy (ADR "the CSP nonce lives in the header"). A post's head code already gets the nonce per request (ADR "head code gets the nonce on each request").

## Decision

- **The site's code lives in `[code]` in `site.toml`:** `head`, checked as head code is; `body`, scripts only; and `consent = "ask"` (the default) or `"not-needed"`.
- **The banner is a plain form.** It posts `choice=yes` or `choice=no` to `<mount>_consent/`. The server answers 303 back to the page with a cookie, `khalid-consent=yes` or `no`, scoped to the mount, `HttpOnly`, `SameSite=Lax`, `Secure` on `https`, kept for 180 days. No script is involved, so it works with scripts off.
- **A page is rendered once, for a reader who has not answered.** It holds the banner between `<!--khalid-ask-->` marks, the form to change a choice inside a `template` between `<!--khalid-chosen-->` marks, a post's own code between `<!--khalid-held-->` marks, and the two slots the site's code goes into. That page keeps its tag, its 304s and its Brotli copy, so new readers and crawlers cost what they cost today.
- **A reader who answered gets a page made for them alone.** The server cuts the banner and shows the change form. For a reader who said no it also cuts the held code. For a reader who said yes it puts the site's code into its slots and fills the nonce, and the policy carries that nonce, as for a post's head code. Such a page has no tag, no Brotli copy, `Cache-Control: private, no-store` and `Vary: cookie`. The kept page says `Vary: accept-encoding, cookie`, so no shared cache hands it to a reader who answered.
- **When the site's code waits, a post's head code waits too.** One rule for the owner: on a site that asks, no code runs before a reader says yes. Verification tags belong in `[seo]`, which never waits.
- **`consent = "not-needed"`** writes the code into every page with the nonce slot, for code that sets no cookie and tracks no one. Every page is then made per request.
- **A static site cannot ask.** A static host cannot keep a reader's choice, so `site.toml` with `mode = "static"` and code that waits is refused at start, with the fix.

## Consequences

- A reader who answered costs one Brotli decode, a few string scans and an uncompressed answer per page. A reader who never answered costs nothing more.
- A held part never closed is cut to the end of the page, so code never runs by mistake. The marks are comments only Khalid writes.
- The search results page is drawn outside the page router and shows no banner yet. `khalid_server::chosen` is public, so it can follow later.
- Not checked on this server: a real browser pressing Allow, since there is no headless browser here. The end-to-end test posts the form and follows the cookie by hand.
