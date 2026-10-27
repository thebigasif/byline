# A saved post's page shows first, and the full render follows beside it

**Number:** next free.

**Status:** Accepted, 2026-10-27. Phase 1 task 1.21.

## Context

The budget for a saved post to show is 50 ms (`docs/standards/performance.md`). After a save, `khalid serve` rendered every page of the site again before swapping any in: about 110 ms on the 1,000-post site for reading every file, turning every post's markdown into HTML and filling every template, plus the Content API's index refresh, which CI measured at 300 ms and more. The CTO answers of 2026-10-27 ask this track to render only what a save changes, and leave the index refresh to the API track.

Which pages one file feeds is not written down anywhere: the home page lists every post, an author page lists theirs, and pages still to come, such as related posts and tag pages, will read more. A dependency map kept by hand would go stale with each new page type.

## Decision

- **`Render::render_saved` renders the saved files' own pages alone.** It has a default that says "cannot tell", so every renderer keeps working. The site's renderer answers for files that are posts the last full render read: it reads that one file again, takes the authors from a body-free outline of the site kept from the last full render, and renders that post's page only. Anything else, such as a new file, an author, a page or `site.toml`, gets no early page.
- **The watch shows those pages at once**, in place of the old ones, without a Brotli copy.
- **A full render always follows, beside the watch, not in front of it.** It brings the home page, the author pages and the Content API up to date, and adds the Brotli copy. When a newer save shows its page while a full render runs, that render is stale: it is dropped and runs again, so a page never flips back to older text.
- **Stopping waits for the full render in progress**, so the database closes after it (BL-RUN-10).

## Consequences

- On the 1,000-post site a saved post shows in about 20 ms, not 340 ms or more. The full render's cost does not change; it just no longer holds the saved page back.
- For a moment after a save, a list such as the home page can still show the post's old title. It catches up when the full render ends.
- A post's page must keep depending only on its own file, the authors and the settings. A page type that adds more to a post page, such as related posts, must either keep that true by reading only the outline, or make `render_saved` say "cannot tell" for it. `a_saved_post_alone_renders_the_page_the_full_render_makes` compares the two on the demo site and fails if they drift.
- The outline holds every post's front matter without its body, and the authors whole, since the author box shows their bios: a few hundred bytes a post.
- Integration with the reader stack and the glossary wikilinks (2026-10-27): related posts are chosen from every post's links, which the outline does not hold, and a wikilink may name any page. So `render_saved` says "cannot tell" when the pages show related posts, which is the default, and when a saved post or an author's bio has a wikilink. The full render then shows the page, as before this decision. Keeping each post's links in the outline would bring the early page back; that is left to the perf and reader tracks.
