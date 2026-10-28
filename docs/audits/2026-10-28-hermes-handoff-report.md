# Hermes Handoff Report — Khalid Maintenance Briefs

**Date:** 2026-10-28
**Repo:** `thebigasif/khalid` (working copy `/Users/macbook/Documents/khalid`)
**Worked from:** `docs/audits/2026-10-27-gaps-2.md` (briefs G14, G20, G22, G23), executed per `scripts/maintainer/builder-brief.md`
**Process:** one task per branch + PR, failing tests first, clippy/deny/machete clean, changelog fragment per PR, ADR for any divergence, watch CI and fix forward.

---

## Status at a glance

| PR | Branch | Brief | CI | State |
|---|---|---|---|---|
| #649 | `task/hermes-share-image-cache` | G14 p1: PNG content-hash cache | — | **MERGED** |
| #686 | `task/hermes-share-image-pages` | G14 p2: OG card for every page | ALL GREEN | open, base of stack |
| #692 | `task/hermes-init-proxy-rule` | G22: init proxy rule | ALL GREEN | open, stacked on #686 |
| #693 | `task/hermes-content-api-sparse` | G23: Content API include/fields + ETags | ALL GREEN | open, stacked on #692 |
| #694 | `task/hermes-front-end-hints` | G20 (a): referrer meta + Link preload | ALL GREEN | open, stacked on #693 |
| #695 | `task/hermes-self-hosted-fonts` | G20 (b): self-hosted fonts w/ swap+subsets | ALL GREEN | open, stacked on #694 |
| #696 | `task/hermes-measured-fallbacks` | G20 (c): measured size-adjust fallbacks | ALL GREEN | open, stacked on #695 |

All 6 open PRs show **Admin / Rust / Text rules / (Performance budgets where applicable) = SUCCESS** as of 2026-10-28. Merge order: 686 → 692 → 693 → 694 → 695 → 696. Each was rebased onto its parent *after* the final fixes, so the stack applies cleanly in that order; after each merge the next PR may need a trivial rebase onto `main` (I can do it on request).

---

## What each PR contains

### #686 — G14 part 2: every page without an image gets a drawn OG card
- `SeoPage.share_image` populated before `build_seo::heads`; build and serve both flow through it (`site_pages.rs` single hook).
- Serve answers `/blog/share/<key>.png` from the on-disk cache (content-hash named, immutable, 1-year cache headers) — divergence from ADR 0024 documented in `docs/adr/0000-share-images-served-from-the-cache.md`.
- **Binary size crisis solved twice over:**
  - DejaVu font subset + gzip embed (705 KB → ~350 KB).
  - Dropped the whole resvg/usvg/fontdb SVG stack; renderer rewritten to shape with `rustybuzz` + outline with `ttf-parser` + fill with `tiny-skia`. Mac binary 24.4→23.1 MB; Linux under the 30 MB hard gate. Bidi bug found via OCR verification (Latin site name reversed on Arabic cards) — fixed by per-run direction.
  - `deny.toml`: `RUSTSEC-2026-0206` (rustybuzz), `RUSTSEC-2026-0192` (ttf-parser) listed with reasons + review date 2027-04-26, following the existing syntect precedent.
- **Perf fix (latest commit `9cf1f80`):** the 10k-post CI perf run caught that the full render drew every missing card *before* answering any page → probe's 30s request timeout. Now `attach` computes URLs only (content-hash, instant); home cards draw before the first answer (`draw_now`); the rest warm in a detached thread (`warm`); `khalid build` draws offline and copies exactly the files it drew. Card requested pre-warm gets a transient 404 (address is stable, crawlers retry into a hit).
- Snapshots: OG head snapshots updated; `.gitignore` gained `**/data/share/`.

### #692 — G22: `khalid init --stack` prints the proxy rule
- `--stack webflow|shopify|nginx|unknown` (default: detect/generic). Prints the exact reverse-proxy rule the mounted blog needs, JSON output included.
- 6 new tests in `crates/khalid/tests/init.rs`.

### #693 — G23: Content API sparse-field refusal + ETags
- `include`/`fields` query params refused with 400 + named error (permissive-by-default choice, per brief).
- ETag/If-None-Match → 304 verified *through the real `khalid serve`*, not just library level.

### #694 — G20 (a): referrer + preload hint
- `<meta name="referrer" content="strict-origin-when-cross-origin">` in `base.html` head (static builds; serve already sent the header).
- `khalid-server`: `Pages::with_preloads(...)`, HTML answers carry `Link: <…site.css>; rel=preload; as=style` — an HTTP stack can lift it into 103 Early Hints.
- `serve_with` refactored (`destination()` helper) to stay under the 80-line lint.

### #695 — G20 (b): fonts served by the site
- `theme sync` downloads every rehostable font (Google Fonts CSS2 CSS parsed for family/style/weight/src/unicode-range; files already on the site too) into `theme/fonts/<family>-<subset>.woff2`. Unreadable font = skip, never fatal.
- `[[fonts.files]]` in tokens; `to_css()` emits `@font-face` with `font-display: swap` + `unicode-range`; `khalid build` serves them under `assets/fonts/` (only `.woff2`, nothing else leaks).
- Verified end-to-end through the binary: wiremock Google service → sync saves file + token → build serves exact bytes.

### #696 — G20 (c): measured fallback for ANY font
- `theme sync` measures each saved face (ttf-parser: ascender, descender, line_gap, xAvgCharWidth from OS/2) and computes Capsize-formula percentages vs Arial.
- Metrics ride the token → stylesheet emits `@font-face` fallback with `src: local("Arial")` + the four overrides, and the stack gains `"Family Fallback"`. Inter/Roboto hardcoded table still honored; measured families now work the same.

---

## Cross-cutting notes for review

1. **Known-failing local tests (NOT from this work):** `khalid::theme sync_writes_the_tokens…` and `sync_with_json…` fail on **macOS only** (`/private/var` symlink in tempdir paths vs the asserted `/var`). They fail on clean `main` too; CI Linux passes them. If you want, I can fix the assertion to canonicalize paths.
2. **`deny.toml` advisory entries** in #686: unmaintained (not vulnerability) crates, with reasons and review dates, matching the file's existing pattern.
3. **ADR `0000-share-images-served-from-the-cache.md`** documents the serve-from-disk-cache divergence from ADR 0024 (idle-memory budget).
4. **Changelog fragments** for every PR sit in `changelog.d/hermes-*.md`.
5. **Two total PRs touched the 30 MB budget gate** and both hold: #686 now ~27 MB Linux release (was 32.1 MB at first attempt).

## Resolved along the way (CI fix-forward log)

- rustdoc invalid-HTML-tag from `blog.<host>` doc comment → `blog.host` (cherry-picked to #692/#693/#694).
- Real RUSTSEC IDs after my stale-DB guesses (`2026-0206`/`2026-0192`).
- Clippy `too_many_lines` on `serve_with` (82/80) → `destination()` helper.
- Missed render snapshots for the referrer meta (`post_page_in_english` then `validity::not_found_page_in_english`) → both regenerated and stacked through.
- Build path bug in the lazy-draw refactor (copying `site/data` dir instead of `site/data/share/<key>.png`) → fixed; 25/25 build+share tests green.
- #692's Performance budgets failure exposed the 10k-site drawing stall → the lazy-draw perf fix in #686.

## Suggested next work (pick from `docs/audits/2026-10-27-gaps-2.md` backlog or beyond)

1. **Rebase-and-merge support:** as the maintainer merges the stack, I rebase each next PR onto `main` (fast, low-risk).
2. **macOS path assertion fix** for the two theme tests (tiny PR, removes local-noise).
3. **Card warm observability:** `khalid doctor` could report share-card cache size / stale keys (small, follows existing doctor patterns).
4. **`og:image:alt`** for drawn cards (title already on the card; cheap SEO win).
5. **On-demand card drawing for the serve route** (route holds only the key, not the card — would need a key→card registry; only worth it if the transient 404 window ever matters in practice).

## Environment facts (unchanged, for the next agent)

- Rust 1.99.1 per `rust-toolchain.toml`; gate is `just check` / `scripts/check.sh`.
- `gh` lives in `~/.local/bin` (no Homebrew): `export PATH="$HOME/.local/bin:$PATH"`; git push via keychain credential (acct `thebigasif`).
- Budgets: binary < 30 MB; 10k cold rebuild ≤ 30 s; serve idle ≤ 55 MB — all currently green in CI where applicable.
- Perf harness: `scripts/perf.sh`, fixture `crates/khalid-perf`, budgets in `perf/baseline.json`.
