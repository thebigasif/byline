# 0024. The server answers from rendered pages in memory

**Status:** Accepted, 2026-10-26. Phase 1 task 1.15.

## Context

Plan section 4.5 says pages render in under a millisecond and are cached until a file changes, with ETags and clean URLs. The budget asks for 10,000 cached requests per second on 2 vCPU, and BL-PERF-2 forbids work at request time. The plan does not say where the cache lives, what an ETag is made from, or how a browser learns that a post was edited.

## Decision

- **Every page is rendered before it is served and held in memory** as `Bytes` in one `Pages` set, keyed by its path once percent-encoding is undone. A request is a map lookup and a cheap clone of shared bytes. Nothing is read from disk or rendered while a reader waits.
- **The ETag is made from the page's own bytes:** the first 128 bits of their SHA-256, in hex, as a strong tag. The same bytes give the same tag on every start, so a browser's copy survives a restart, and any edit that changes the page changes the tag.
- **Pages are sent with `Cache-Control: no-cache`.** Browsers may keep them but check the tag first. A current copy costs a 304 with no body, and an edited post shows on the next visit. Long-lived caching is for assets with hashed names, not pages.
- **`If-None-Match` is compared weakly** (RFC 9110 section 13.1.2), so `W/"x"` and lists match. The not-found page carries no tag and never gets 304.
- **One address per page.** `/post` and `/post/index.html` answer 308 to `/post/` when that page exists. A page path cannot start with `//`, and redirects only go to pages, so no redirect leaves the site.

## Consequences

- Memory grows with the site. A post page is 10 to 30 KB of HTML, so 1,000 posts take 10 to 30 MB, and the 50 MB idle budget holds only while pages stay lean. Task 1.21 measures it. Precompressed copies (Brotli, next part) add about a fifth again. A site far larger than the budgets' fixtures may need a disk tier later. That would be a new ADR.
- A change to the content replaces the set as a whole, so a reader never sees half a rebuild.
- Hashing every page at publish costs about a microsecond per kilobyte, once per change, never per request.
