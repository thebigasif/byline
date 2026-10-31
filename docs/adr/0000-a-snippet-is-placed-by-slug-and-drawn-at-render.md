# 0000. A snippet is placed by its slug and drawn when the page renders

**Status:** Proposed, 2026-10-31. Number: next free. Gap brief G50, third part (KH-EDT-1, KH-CON-1).

## Context

Plan section 4.6 asks for "snippets that save a group of blocks and update everywhere". Snippets are already files, `content/<lang>/snippets/<slug>.md` (KH-CON-1), and the G24 Admin API reads and writes them. Nothing placed one in a post: no block named a snippet, so the editor had nothing to write and the renderer nothing to draw. Copying the snippet's markdown into each post would not update everywhere.

## Decision

A post places a snippet with a one-line leaf directive, as ADR 0018 writes blocks with no body:

```
::snippet{slug="affiliate-disclosure"}
```

The post's file keeps only that line. The renderer draws the snippet's blocks in its place when the page renders, from the snippets of the post's own language, which `Wikilinks::for_language` now gathers beside the wikilink targets, because every caller that resolves wikilinks already holds that per-language lookup. A change to the snippet file therefore shows in every post on the next render, and the content files stay the source of truth.

- `check_bodies` refuses a snippet slug that the post's language does not have, with the file and line, so a page never has a silent gap.
- A snippet placed inside a snippet draws nothing, so a snippet can never place itself forever.
- `render` without a site, which knows no snippets, draws nothing for the block. The Content API's `html` field is one such caller today.

## Consequences

- One more block name, so `BLOCKS` and the editor's directive specs carry 24.
- Lint input renders through the same lookup, so a snippet's words and links count in each post that places it.
- The Content API's rendered `html` leaves snippets out until it renders with the site's lookup; the markdown it returns keeps the block.
