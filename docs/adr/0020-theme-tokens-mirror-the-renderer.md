# 0020. Theme sync writes tokens by the renderer's rules, without depending on it

Renumbered from 0013, which ADR 0013 on SEO outputs holds.

**Status:** Accepted, 2026-10-26. Phase 3 work pulled forward, part 3.3.

## Context

`khalid-render` reads `theme/tokens.toml` with fixed tables (`colors`, `fonts`, `shape`), checked values and no unknown field. `khalid-theme` writes that file. `docs/standards/rust.md` says dependencies point down its crate table, and `khalid-render` sits above `khalid-theme`, so the theme crate may not depend on the renderer.

## Decision

- `khalid-theme` keeps its own copy of the shape and the value rules: colours `#rgb` or `#rrggbb`, font lists of names, commas, spaces, hyphens and paired double quotes up to 300 bytes, lengths in `px`, `rem`, `em` or `ch`, and the same light defaults. A fixture holds the defaults file byte for byte.
- The preview page is rendered by the binary, which may use both crates.
- A field the renderer does not read yet, such as the `locked` list for re-sync, is added to both crates together.

## Consequences

- Two copies of a small rule set must agree. The fixture and the renderer's own tests catch a drift.
