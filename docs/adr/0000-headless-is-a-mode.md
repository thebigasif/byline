# 0000. Headless is a third `mode`, not `theme = false`

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 1, audit brief B-INS-5 (KH-INS-5).

## Context

KH-INS-5 asks for a headless mode: the theme switched off and everything else running, for sites that render their posts in their own front end, such as the Astro and Next.js starter kits of Phase 7. The audit brief B-INS-5 leaves the setting's name to an ADR: `theme = false` or `mode = "headless"`. `site.toml` already has `mode`, with `server` (Khalid runs the blog) and `static` (Khalid writes plain HTML to upload, and server features are locked). Nothing reads `mode` yet beyond `khalid init`.

## Decision

- **Headless is a third value of `mode`:** `mode = "headless"`. `Mode::Headless` sits beside `Server` and `Static` in `khalid_core::Config`, and `Mode::has_pages` is the one question the build and the server ask.
- **Why not `theme = false`.** A flag beside `mode` makes four combinations, and one of them is nonsense: `mode = "static"` with no theme writes no pages and runs no server, so nothing could read the posts. One enum with three values makes that state impossible to write, and RULES.md BL-TYP-1 asks for an enum, not a `bool` that means a mode. The word "theme" also means theme sync in this project, which copies the main site's colours and fonts; `theme = false` reads as "do not sync" to a person who has met that feature first.
- **What headless keeps.** `khalid serve` answers the Content API, the Admin API, the feeds, the sitemaps, `robots.txt`, `llms.txt` and the alias redirects. Every HTML page answers 404. `khalid build` writes the same files and no HTML and no stylesheet. Addresses in sitemaps and feeds stay the ones the routes give, because the front end is expected to render each post at the same address under the same mount.
- **The theme is never loaded** in headless mode, so a site with no `theme/` folder, or a broken template, still serves and builds.
- **The 404 body is a short fixed page** saying the pages are shown by another app, with no theme and no stylesheet.
- **`khalid init` still asks only server or static.** Headless is for developers wiring their own front end, who edit one line of `site.toml`; a third choice in the first-run question would ask every writer about something only developers need.

## Consequences

- A headless site's sitemaps and feeds point at addresses Khalid does not answer itself. The starter kits must render at those addresses, and their docs say so in Phase 7.
- `static` and `headless` both lock something; a site that wants static files and no HTML has no mode. Nobody has asked for that, and feeds without pages have no reader.
- Later features that only make sense with pages, such as the theme preview, check `Mode::has_pages` rather than the mode's name.
