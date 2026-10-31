# The reader script is one hashed file, allowed by its hash, so cached pages stay the same bytes

**Number:** next free.

**Status:** Proposed, 2026-10-27. Phase 1 task 1.23. Needs the server track for the last step.

## Context

Plan section 4.12 adds polish that needs JavaScript: copy a heading's address, open an enlarged image in a `dialog`, open search with `/` or Ctrl+K, and mark the section being read. The CTO's rule for task 1.23: "any script is one small deferred file under the CSP nonce, no inline handlers, and every page must read completely with scripts blocked."

The ADR "the CSP nonce lives in the header" keeps rendered pages byte for byte the same across requests, for ETags, 304s and the precompressed copies. A nonce printed in the page would break that. That ADR says the first script needs a new ADR.

## Decision

- **One file.** `khalid_render::reader_script()` builds `assets/reader.js` as a hashed `Asset` (`reader.<hash>.js`), like the stylesheet, with its gzip and Brotli copies made once. It is plain JavaScript with no dependencies, under 4 KB compressed (a test holds it there), and it writes no markup from strings.
- **One tag, no nonce in the page.** Pages print `<script src="…/reader.<hash>.js" integrity="sha256-…" defer data-…>` only when the wiring sets `RenderSite.script`. Its words come from the page language through `data-` attributes. There are no inline handlers and no inline script.
- **The policy allows it by its hash.** The server adds the script's `Asset::integrity()` value to `script-src`, next to the per-request nonce: `script-src 'nonce-…' 'strict-dynamic' 'sha256-…'`. CSP level 3 lets a hash source allow an external script whose `integrity` attribute carries that hash. The page bytes never change per request, and the browser runs only those exact bytes.
- **Everything works without it.** Every feature it improves is a plain link first: the heading `#` link, the link to the largest image, the search link and the contents column. A browser that blocks the script, or does not match hashes to external scripts, shows the plain version.

## Consequences

- `khalid serve` answers `reader_script()` beside the stylesheet and adds its hash with `SecurityHeaders::with_script`, and `khalid build` writes it to `assets/` (G19). Pages link it whenever one of the features it improves is on. `MediaType::for_file` never gives `JavaScript`, so a file a writer puts beside a post is never served as a script.
- A browser that does not match a hash source to an external script blocks it, and the page reads as it does with scripts off.
- Each change to `reader.js` changes its name and its hash together, so a page, its policy and the file always agree.
- Not checked on this server: the script's behaviour in a browser, since there is no headless browser here. The six-width and scripts-off CI runs are where it gets exercised.
