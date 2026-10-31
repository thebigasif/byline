# 0000. The editor links posts with wikilinks, through its own small menu plugin

**Status:** Proposed, 2026-10-31. Number: next free. Gap brief G50, second part (KH-EDT-1).

## Context

Plan section 4.6 asks for a slash menu, markdown shortcuts, link search with `cmd+k`, `@` to mention a post, tag or author, and wikilinks. Tiptap sells this as `@tiptap/suggestion` and `@tiptap/extension-mention`, which are not in `admin/package-lock.json`, and the lock file cannot be regenerated on the build server. A link to a post also needs an address, and a post's address depends on `routes.toml`, which the admin cannot read.

## Decision

- **One small plugin of our own** (`admin/src/editor/aids/trigger.ts`) watches the text before the cursor for `/`, `@` and `[[`, and keeps the open menu in plugin state. The menus are React components that read that state. No new dependency.
- **`/` opens the block menu only on an empty line.** A slash in a sentence, such as `1/2` or a web address, never opens it. The menu holds the starter kit's blocks and every directive of plan 4.6. Picking a directive opens a form with its details, one labelled field per attribute, before the block is added.
- **Every link to a post is a wikilink.** `@` inserts `[[slug|Title]]`, `[[` inserts the bare `[[slug]]` that the build shows as the post's title, and link search turns the selected words into `[[slug|words]]`. `khalid-content` resolves a wikilink to the post's address at build and lints a broken one, so the link follows the post when its address or the site's routes change. A web address typed into link search becomes a plain markdown link.
- **`@` mentions posts only, for now.** The Admin API lists posts for a staff token, but tags and authors are only in the Content API, which needs a content key. Mentions of tags and authors wait for an Admin API list of them.
- **Wikilinks are an inline node** that prints `[[slug]]` or `[[slug|text]]` exactly as written, so a post with wikilinks opens in the rich-text view.

## Consequences

- A post written in the editor has no hard-coded addresses to its own site, so a later change of `routes.toml` breaks no link.
- The plugin is about 90 lines we own and test. If the lock file can be regenerated later, swapping it for `@tiptap/suggestion` touches only the menus.
- Mentioning a tag or an author is a known gap, written in the pull request.
