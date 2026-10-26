# 0013. SEO outputs are built from their own input types

**Status:** Accepted, 2026-10-26. Phase 1 tasks 1.13 and 1.14.

## Context

`crates/khalid-seo` writes the head tags, hreflang, sitemaps, feeds, `robots.txt`, redirects and schema graphs. The content and render crates it reads from are being built at the same time by other sessions, and their types are still moving. The plan also sets defaults the code has to choose numbers for: which archives are thin, and what a page says to robots when nobody set anything.

## Decision

- `khalid-seo` defines its own inputs: `SeoSite` for the site, `SeoPage` for a page, and more for authors, feeds and schema later. It depends on `khalid-core` only. Task 1.15 builds these inputs from the content tree and `Config`.
- Every output is an ordered value (`Vec` or `BTreeMap`) with a `to_html`, `to_xml` or `to_string`, so the same input always gives the same bytes, and snapshots stay stable (BL-TYP-5).
- A page's own title, description, robots directives and canonical address replace the site defaults whole. They are never merged field by field, so a writer sees exactly what they set.
- Default robots content: `index, follow, max-snippet:-1, max-image-preview:large, max-video-preview:-1`, which lets Google show full snippets and large images in Discover. A `noindex` page says only `noindex` and its follow setting.
- Default index policy: search results, date archives and the not-found page are `noindex`. A tag or author archive with fewer than three posts is `noindex` as a thin archive. Each setting is a field a site can change.

## Consequences

- The SEO crate is tested on its own with the demo site written as inputs, and it cannot break when another crate renames a field.
- Task 1.15 carries a small mapping from content types to SEO inputs. That mapping is the one place to update when content types change.
