# 0016. The share image is drawn in pure Rust, in its own crate

**Status:** Accepted, 2026-10-26. Phase 3 work pulled forward, task 3.7.

## Context

PRD KH-SEO-2 asks for a generated share image when a post sets none: the picture a link preview shows on social sites and in chat apps. It must be a 1200 by 630 PNG in the theme's colours, drawn at publish time, never at request time (BL-PERF-2), on every machine the single binary runs on, with no headless browser and no system library. Titles come in every language the site uses, including Arabic, which needs shaping and right to left order.

## Decision

- **A crate of its own, `khalid-share-image`.** Drawing pulls in an SVG reader, a text shaper and a rasteriser. Keeping them out of `khalid-theme` keeps theme sync light, and keeps the two pieces of work in separate pull requests. The crate takes a `ShareCard` (title, site name, three colours, direction) and knows nothing about content, templates or tokens on disk.
- **`resvg` with only its `text` feature.** The card is laid out as SVG and drawn with `resvg` and `tiny-skia`, all Apache-2.0 or MIT and pure Rust. The `text` feature brings `harfrust` for shaping and `unicode-bidi` for direction, so Arabic letters join and right to left titles are set from the right. System fonts, compressed SVG and raster images are left off.
- **One embedded font, DejaVu Sans Bold,** from the `dejavu` crate. Embedding it means every machine draws the same pixels, so the snapshot test holds in CI, and nothing on the host is read. DejaVu covers Latin, Greek, Cyrillic, Hebrew and Arabic.
- **Line widths are measured by the same shaper that draws them.** The title wraps at spaces to three lines, and a longer title, or a single word wider than a line, is cut with an ellipsis, so nothing runs off the card.
- **Cached by content key**, in the second part of task 3.7. The file is `data/share/<key>.png`, where the key is the SHA-256 of a layout version and every field, each with its length first. A changed title or colour gives a new file, and the old one is never served for it. Changing the layout means changing the version string. Writes are atomic (BL-RUN-9).

## Consequences

- The binary grows by the font, about 700 KB, plus the shaper and the rasteriser. The release size check measures it when the crate is wired into the binary, against the 30 MB budget.
- DejaVu has no Chinese, Japanese or Korean glyphs, so titles in those scripts cannot be drawn yet. Adding a CJK font is a later, separate decision, because each such font adds several megabytes.
- Drawing one card takes a few milliseconds of CPU, so callers on the async runtime run it in `spawn_blocking` (BL-RUN-7).
