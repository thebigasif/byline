# 0000. The Content API carries each post's head as ordered tags

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase 7 brief G55 (KH-INS-5).

## Context

G55 asks for Astro and Next.js starter kits that render the demo blog from the Content API "with schema and meta intact". Until now a post in the API had its title, address, dates and bodies, but not the tags its page carries in `<head>`: the search title, description, robots, canonical, hreflang, Open Graph, the Twitter card and the JSON-LD graph. A kit would have to rebuild all of that in JavaScript, and it would drift from what `khalid-seo` writes.

## Decision

- **Every post in the Content API has a `head`:** `tags`, the title, meta and link tags in the order the theme writes them, and `json_ld`, the schema graph as compact JSON with each `<` escaped, ready to go inside a script tag.
- **The head comes from the render, not from the request.** `khalid serve` already makes every post page's SEO page on each full render, in headless mode too, for the sitemaps and feeds. When the site has a Content API, the render also makes each post's head from those same SEO pages with `khalid_seo::head_tags` and `page_graph`, and hands them to the API store with the catalog. Nothing is computed per request (BL-PERF-2).
- **Tags, not an HTML string.** A Next.js app cannot drop raw HTML into `<head>`, but React 19 hoists `<title>`, `<meta>` and `<link>` elements written anywhere. Ordered tags with plain-text values serve both kits, and a front end that escapes the five characters `khalid-seo` escapes writes the theme's head byte for byte. A test in `khalid-api` holds that.
- **Always sent, on lists and single posts.** The head is a few kilobytes and is held behind an `Arc`, so the store copies a pointer, not the text. A `formats` or `include` switch was considered; `formats` names body formats and `include` is refused until G23, so a plain field is the least surprising.
- **Posts only for now.** Pages, glossary terms and the other entry types come from a separate store and keep their current shape.

## Consequences

- A front end of its own keeps the same meta tags and JSON-LD the built-in theme writes, and they change when `khalid-seo` changes.
- The full render of a site with an API makes each post's head twice: once as HTML for the page, once as tags for the API. Both are string building over data already in memory.
- A head tag kind added to `khalid_seo::HeadTag` later is left out of the API until `PostHead` maps it; the byte-for-byte test fails first.
