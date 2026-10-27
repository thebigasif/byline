# khalid serve answers before the full render

**Number:** next free.

**Status:** Accepted, 2026-10-27. Phase 1 task 1.21.

## Context

The budget for `khalid serve` to send its first byte is 100 ms on the 1,000-post site, with 10% drift (`docs/standards/performance.md`). The server rendered every page of the site before it listened: every post's markdown, its related posts, every list, every head and the sitemaps and feeds. CI measured 205 ms to the first byte, most of it that render. The CTO answers of 2026-10-27 ask that the first request be answered before everything is built, by rendering lazily or in the background.

Rendering one page on request would need a map of what each page reads, which the related posts, the author box and the lists make wide, and a page that went wrong would surface only when someone asked for it. The full render already exists and is checked; it only has to stop being in front of the first answer.

## Decision

- **Before it listens, `khalid serve` renders only the first pages**: the home page of each language and the stylesheet it links (`SitePages::render_first`). The home page lists every post, but only needs their front matter, so no body is rendered for it (`build_home`). Each is byte for byte the page the full render makes.
- **The server starts with those as `Pages::first`.** A request for any other address waits for the whole set, then gets the page, the redirect or the 404 the whole site gives. A reader never sees a 404 for a page that is about to exist.
- **The full render runs right after the server listens**, on a blocking thread, from the content already read and checked. It replaces the first pages, then gets its Brotli copies as before. A save during the start that already served a whole set wins.
- **A full render that fails stops the server** with the same message, file and exit code 1 it gave when it rendered before listening, so a broken image still stops `khalid serve` instead of leaving half a site up.

## Consequences

- On the 1,000-post site the first byte comes in about 190 ms on the busy perf-track server, against about 700 ms before, and at about a third of that in CI.
- For the first moment, a request for a post waits as long as it waited before for the whole server to start; no request gets an answer later than it did.
- The server now prints where it listens before it knows the whole site renders. A site whose pages cannot render prints that line, then stops with the error.
- A page type that the home page shows must keep reading only the posts' front matter, the authors and the settings, or `render_first` must render it too. `the_first_pages_are_the_full_renders_home_pages_and_stylesheet` compares the first pages with the full render's on the demo site and fails if they drift.
