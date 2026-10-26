# The SEO head reaches templates as trusted HTML

Number: next free

**Status:** Accepted, 2026-10-26. Phase 1 tasks 1.13, 1.14 and 1.16.

## Context

`khalid-seo` writes every SEO tag of a page in a fixed order: title, description, robots directives, canonical, hreflang, the verification tags on the home page, Open Graph and the Twitter card (ADR 0013). It also writes the page's JSON-LD graph as one script tag that no text can close. The base layout in `khalid-render` already prints a smaller set of these tags from `PageMeta`, with no JSON-LD. `khalid-render` must not know about `khalid-seo` (ADR 0009), and the two must not print the same tag twice.

## Decision

`PageMeta` gains `seo_head: Option<HeadHtml>`. `HeadHtml` is HTML that `khalid-seo` built with every value escaped, so templates print it as it is, the same way `BodyHtml` carries a sanitised body.

- When `seo_head` is set, the base layout prints it in place of its own title, description, robots, canonical, hreflang and share card tags. The charset, the viewport and the stylesheet link stay in the layout.
- When it is not set, the layout prints its own tags as before. Render tests and theme previews keep working without the SEO crate.
- The wiring in the binary builds the head from `khalid_seo::head_tags` and `khalid_seo::page_graph` for every page it renders.

## Consequences

- One place decides what a page's head says: `khalid-seo`. A theme that replaces `base.html` prints `meta.seo_head` to keep it.
- The JSON-LD is a data block, not a script that runs, so the Content Security Policy needs no exception for it.
- The `PageMeta` fields the layout printed before stay, because the language switcher and menus still read `alternates` and `path`.
