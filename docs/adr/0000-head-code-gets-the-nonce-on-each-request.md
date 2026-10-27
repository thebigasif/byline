# Head code gets the nonce on each request, on its own post's page only

**Number:** next free.

**Status:** Proposed, 2026-10-27. Phase 5 task 5.11 and plan section 4.6 (per-post head code, KH-EDT-3).

## Context

A post may carry head code: HTML for its page head, such as a site verification tag or an analytics snippet. The plan's feature table says GA4 arrives through this field. Such snippets are a script tag that loads a file and an inline script that sets it up.

The ADR "the CSP nonce lives in the header" allows no inline script on any page, and keeps cached pages the same bytes. It says the first page that needs an inline script needs a new ADR, and sketches the answer: the page marks the spot and the server fills it per request for that page only. A hash in the policy would allow the inline script but not the file it loads, because `'strict-dynamic'` makes browsers ignore host lists, and analytics scripts send their data with `fetch`, which `connect-src 'self'` blocks.

## Decision

- **Only owners and admins set head code** through the Admin API, and `HeadCode` checks it first: only `script`, `style`, `meta` and `link`, every attribute quoted again, no `base`, no `meta http-equiv`, no event attributes, nothing left open.
- **The page marks the spot.** When `khalid serve` renders a post with head code, each `script` and `style` of the head code gets `nonce="khalid-nonce-slot"`, after the SEO head. `khalid_server::Page::with_nonce_slot` holds that page.
- **The server fills it per request.** For that page alone, each answer replaces the slot with the request's nonce, 32 hex digits from the operating system, and sends a policy with the same nonce. Hex digits cannot open a tag, so the replacement cannot inject anything. Without random bytes the slot is emptied and the policy allows no script, which is stricter, never looser.
- **The policy opens only what analytics needs, only on that page:** `style-src 'self' 'nonce-…'` and `connect-src 'self' https:`. `script-src` stays `'nonce-…' 'strict-dynamic'`, so a script runs only when the head code or a script it loaded asked for it.
- **The page is never kept.** It has no `ETag`, no Brotli copy and `Cache-Control: no-store`, because no two answers are the same bytes.
- **A static build writes the head code without a nonce.** A static host sends its own policy, or none, and Khalid cannot fill a slot there.

## Consequences

- Every other page keeps the strict policy, its tag, its 304s and its precompressed copy. A post with head code costs one string replace and no compression per request; the page is still rendered once, at publish.
- An owner who adds a script that loads from a host outside `https:` sees it blocked, which is the policy working.
- Not checked on this server: that GA4 loads and reports in a real browser, since there is no headless browser here. The server test proves the page and its policy carry the same fresh nonce.
