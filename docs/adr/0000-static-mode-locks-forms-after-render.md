# 0000. Static mode locks forms after the page is rendered

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 5, static mode (KH-OPS-1).

## Context

With `mode = "static"`, `khalid build` writes files any host can serve. A static host only sends files, so nothing answers a form. Today a static build still writes the subscribe block, gated content and the contact form block, which post to `/subscribe` and `/forms/<id>`. A reader who fills one in gets an error page. The theme's search form and a future comment form have the same problem.

The forms come from two crates owned by other tracks: `khalid-content` writes the blocks into a post's body, and `khalid-render` writes the theme's pages. Teaching each of them about the mode spreads one rule over several crates, and every new form would need to remember it.

## Decision

- **A new crate, `khalid-static`, owns static mode.** It names the features a static site cannot have (`ServerFeature`: comments, newsletter, forms, search, the admin and scheduled publish) and rewrites a rendered page.
- **The rewrite runs on the finished HTML**, once per page in the build. Every `<form>` whose `action` is on the site itself, or missing, is replaced by one short sentence in the page's language: `<p class="locked" data-feature="newsletter">Signing up by email is not open on this site.</p>`. A form that posts to another site, such as a newsletter provider, or opens an email, stays, because it works on any host.
- **The scan is plain string work, not an HTML parser.** Rendered pages escape `<` in text, code and JSON-LD, so every `<form` is a real tag, and forms cannot nest. A form with no closing tag stops the build with an error instead of guessing where it ends.
- **Readers and owners read different words.** The reader's sentence says only that the feature is not open, because a reader can do nothing about it. The owner's message (which feature is off, why it needs a server, how to switch) is a `UserMessage` the binary makes for `khalid build` and `khalid doctor` (BL-ERR-4).
- **The reader's sentences live in `crates/khalid-static/strings/`, one file per language**, with English filling in for a language with no file, the same way the render crate's strings work. They are not in `khalid-render/strings/`, which another track owns. When the templates track wants one strings table, the four keys move there unchanged.

## Consequences

- Any new form, in a block or a template, is locked in static mode without its author doing anything.
- The note carries the class `locked`. The default stylesheet has no style for it yet, so it shows as a plain paragraph until the templates track adds the grey card.
- A theme that writes its own provider form keeps it. A theme that writes a form to a relative address on another server, which nobody does today, would be locked by mistake; it can use a full address instead.
- The scan reads every page's text once more, in static mode only. Server mode skips it, so server builds and `khalid serve` cost nothing more. A static build still answers to the full-rebuild budget of 30 seconds for 10,000 posts.
