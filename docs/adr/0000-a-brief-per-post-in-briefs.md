# 0000. A post's brief lives in briefs/, and the linter holds the post to it

**Number:** next free. **Status:** Proposed, 2026-10-31. GEO phase, task P6, an optional brief per post (KH-LNT-2, KH-LNT-3, KH-LNT-6). Taken under the founder's yes in advance of 2026-10-27; the phase brief gives P6 one line and no done line, so this record sets the scope.

## Context

P6 asks for an optional brief per post. Koray Tugberk Gubur teaches writing from a content brief made before the text: the questions the page answers, in the order a reader asks them and written as its headings, the entities and their attributes a full answer covers, and the internal links planned between pages of the topic. Steve Toth teaches building a page around the questions people actually ask, each answered where a reader and an answer engine can find it. Both treat the brief as the plan the finished page is checked against.

Neither says how a brief file is laid out, how a question is matched to a heading, or how strict the check should be. Those details are our own calls, listed below.

## Decision

- A brief is a TOML file at `briefs/<language>/<slug>.toml` in the site folder: the post's language folder, then the last part of its address. It holds three lists, `questions`, `terms` and `links`, and an optional `notes` text for the writer that nothing checks. Every key may be left out.
- Briefs are person-written, so they live in the site folder beside `facts.toml` and `lint.toml`, not in `data/`. They stay out of `content/`, so the content reader and `PostFront` (the content track's) are untouched, and a brief can be written before its post exists.
- The focus keyword stays in the post's front matter. A brief does not repeat it.
- The binary reads `briefs/` once per lint, build or publish and refuses a broken brief, a `.toml` outside a language folder, or a name that is not a language or an address, with what, why and fix. Other files, such as a README, are left alone. A missing folder means no briefs.
- The linter gets a brief as `LintPost::brief`, so SerpDad's backend can pass its own. Three rules read it, each one finding per missed entry, in the brief's order:
  - `brief-question-unanswered`: no heading holds every word of the question, in any order, ignoring letter case and punctuation.
  - `brief-term-missing`: the body and headings never use the term as whole words in order.
  - `brief-link-missing`: no link in the post reaches the page, ignoring a last slash, a `#part` or a `?query`.
- All three warn. A brief is optional and a plan can change while writing, so a missed entry never blocks by default. A site that wants a hard stop sets a rule to `block` in `lint.toml`, and the build and the publish door then refuse.

## Consequences

- A writer, or an AI writer working through the API, can set the plan first and run `khalid lint` to see what is still missing.
- A brief whose post does not exist yet is read and kept, and simply matches nothing. Khalid does not warn about it.
- The Admin, the API and the MCP server do not read or write briefs yet; each is a follow-up. `khalid new` does not write a starter brief.
- The demo site has no briefs, so its lint and audit results do not change.
