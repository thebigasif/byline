# 0000. The share image is served from the cache folder, not from memory

**Number:** next free

**Status:** Proposed

## Context

ADR 0016 draws each page's share image at publish time and keeps it in `data/share/<key>.png`. ADR 0024 says the server answers every page from one in-memory set of rendered bytes, and nothing is read from disk while a reader waits. The idle memory budget is 50 MB for 1,000 posts, and it is already tight: page HTML alone takes 10 to 30 MB.

A 1,000-post site draws around 1,000 cards, each a PNG of 20 to 30 KB. Held in the page set that is another 20 to 30 MB, which the budget cannot hold. Brotli copies, which the page set adds to text pages, do not help a PNG either.

## Decision

- `khalid build` copies each drawn card from `data/share/` into `out/share/<key>.png`, beside `assets/` and `media/`. A static host serves the file as it serves the stylesheet.
- `khalid serve` answers `<mount>/share/<key>.png` from its own route, reading the file from `data/share/` on the request. The card is never part of the in-memory page set.
- The reading runs in `spawn_blocking` (BL-RUN-7). The key is parsed, never used as a path segment unchecked, so a request cannot escape the cache folder.
- The answer is immutable for a year: the key in the address is the hash of the card's content, so any change gives a new address and the old one simply stops being linked.

## Consequences

- A card request pays one small disk read. That is the one deliberate exception to ADR 0024's "nothing is read while a reader waits", taken because the alternative breaks a budget that CI gates. A card is a few kilobytes and the read is a page-cache hit after the first ask.
- The idle memory budget is untouched by cards, whatever the site's size.
- A card drawn by a build is answerable by a serve on the same folder with no extra work, because both read the same `data/share/`.
- If a cache file is deleted while the server runs, that card 404s until the next render draws it again. The render always draws missing cards, so the window is one render long.
