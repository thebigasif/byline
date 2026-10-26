# 0022. One linked stylesheet, no inline critical CSS

**Status:** Accepted, 2026-10-26. Phase 1 task 1.12.

## Context

The phase brief for 1.12 asks for "one public stylesheet under 20 KB compressed, critical CSS inline". Inlining the CSS a page needs first is a common way to save a request on a large stylesheet. But every page also carries a Content Security Policy with a per-request nonce (BL-SEC-2), and pages are rendered at publish and cached (BL-PERF-2).

## Decision

The public site links one stylesheet, `site.<hash>.css`, and inlines nothing.

- The whole stylesheet is about 6.7 KB, and 1.8 KB with Brotli. There is no large file to split, and the "critical" part would be most of it.
- An inline `style` element needs a nonce or a hash in the CSP. A nonce changes per request, so a cached page would need rewriting on every request. A hash would have to track every theme change.
- A linked file with a hashed name is cached for a year after the first page, so every later page needs no CSS request at all.
- No `style` attribute appears in any template either, and the page validity test refuses one.

## Consequences

- The CSP can say `style-src 'self'` with no exception.
- If the stylesheet ever grows near its budget, this record is revisited with measurements from Lighthouse, not guesses.
