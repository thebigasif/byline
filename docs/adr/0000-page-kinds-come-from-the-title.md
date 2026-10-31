# Comparison, alternatives, best-of and review pages are read from the title

**Number:** next free. **Status:** Proposed, 2026-10-31. GEO phase task P4, `docs/phases/phase-geo.md`.

## Context

P4 asks for "comparison, alternatives, best-of and review templates" and nothing more. The research behind the GEO list is not in the repository, so this record fixes what the task builds.

The idea comes from Koray Tugberk Gubur's query templates and document templates: searches that repeat one pattern, such as "X vs Y" or "best X", are answered by pages that share one shape. Steve Toth's answer-first guidance, which W1 already follows, puts the verdict where a reader and an answer engine look first. Neither source lists title words, says which shape each kind needs in a form code can check, or gives limits. Those are this project's choices, recorded here.

## Decision

- **The title says the kind.** `PageKind::of_title` in `khalid-lint` reads four kinds: a comparison (`vs`, `vs.` or `versus` with a name on each side, in any language), alternatives, a review, and a best-of list (English and German words). When a title holds words of two kinds, alternatives win over a review and a review over a best-of list. A number from 2 to 100 in an alternatives or best-of title is the count of picks it promises; a year is not.
- **No new front matter field.** Front matter belongs to the content track, and the title already states the search the page answers. A post whose title names no kind is left alone.
- **Four warnings, never blocks.**
  - `comparison-without-table`: a comparison, alternatives or best-of post with no table.
  - `comparison-answer-late`: a comparison whose text before the first heading never names one of its two sides. A side is named when one of its words appears, endings allowed, as in P9.
  - `fewer-picks-than-title`: a numbered alternatives or best-of title promising more picks than any one heading level holds. More headings than picks pass, because a list also has an introduction and a method section.
  - `review-without-rating`: a review with no review block, so it gives no rating and no `Review` structured data.
- **The lint model counts tables and review blocks.** `LintPost` gains `tables` and `reviews`, filled by `khalid` from the rendered body.
- **`khalid new --template`** writes a starter for each kind. It refuses a title that does not read as the kind asked for, so the starter and the checks always agree.

## Consequences

- A comparison written as "Ahrefs or Semrush" is not read as one: "or" and its German word are too common in other titles.
- Titles in languages other than English and German get only the comparison kind until their word lists are added.
- The Admin and the MCP server do not offer the starters yet. The Admin needs a mockup first.
