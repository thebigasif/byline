# Related posts are scored from shared links, category and tags, in the render crate

**Number:** next free.

**Status:** Proposed, 2026-10-27. Phase 1 task 1.23.

## Context

Plan section 4.12 asks for three to five related posts under each post, "chosen at build time from shared category, shared tags and wikilinks in both directions, newest first on ties. Deterministic, so the same build gives the same list. A post can pin its own list in front matter." It does not say how much each signal weighs, what happens when a post shares nothing, or which crate chooses.

## Decision

- **Scores.** A link between the two posts, in either direction, counts 3: a wikilink, or a plain link to the other post's address, which says the same thing. The same category counts 2. Each shared tag counts 1. A link is the writer saying "read this next", so it weighs most; a category is broader than a tag, but one category outweighs one tag because a post has one category and many tags.
- **Order.** Highest score first, then newest, then slug. The slug makes the order total, so the list never depends on the order posts were read in. A test shuffles the posts and gets the same list.
- **Length.** The best five that score. With fewer than three, the newest other posts fill the list to three, so every post offers somewhere to go next. A site with one post shows none.
- **Pins.** When `related` in front matter names any existing post, those posts are the list, in the writer's order, at most five, without the post itself.
- **Where.** `khalid_render::related` takes plain `PostTopics` records (slug, date, category, tags, linked posts) and returns indexes. The render crate owns what the default templates show, and the plain records keep it free of the content tree (ADR 0009). The wiring builds the records once per build and maps the indexes to post cards.

## Consequences

- Choosing is O(n²) over the posts of one language at build: 1,000 posts make a million cheap comparisons, well inside the build budget. A site of tens of thousands of posts would want an index by category and tag. That is a measured change for later, not a guess now.
- The weights are constants in one file. Changing them changes every list at the next build, which is safe because nothing derived is stored in `content/`.
