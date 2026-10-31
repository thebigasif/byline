# The page or section check matches subject words, and reuses the thin-post length

**Number:** next free. **Status:** Proposed, 2026-10-31. GEO phase task P9, `docs/phases/phase-geo.md`.

## Context

P9 asks for a "page or section check" and nothing more. The research behind the GEO list is not in the repository, so this record fixes what the check does.

The idea comes from Koray Tugberk Gubur's macro and micro context: a page has one main context, and its sections are sub-contexts of it. A sub-topic that leaves the main context belongs on a page of its own, and two pages on one context compete. Steve Toth's writing guidance, which W1 to W4 follow, is about answers and passages inside a page; it does not cover this split. Neither source gives a word count, a way to tell two subjects apart in code, or a site-wide rule for merging posts. Those are this project's choices, recorded here.

## Decision

- **Two directions.** `section-wants-own-page` (a post rule) warns about a `##` section that has left the post's subject. Two site rules warn where a heading names the whole subject of another post in the same language, and say which should give way:
  - `post-fits-section`, on the other post, when it is shorter than the thin-post length: it belongs as that section.
  - `section-repeats-post`, on the post with the heading, when the other post is not known to be short and is not linked from it: the section should shrink to a summary and link there. A post that does not give its length is never called short.
- **A post's whole subject** is its focus keyword, or its title when it has none. A heading names it when the heading holds every one of its words. A one-word subject must also be the heading's only subject word, so `links` does not claim every heading about links. These are site rules, not post rules, so the comparison runs once per site through an index of subjects by their first letters, not once per post against every post.
- **A subject is its words.** A post's subject is the words of its title, search title, focus keyword and secondary keywords, without filler words and question words (`how`, `warum`). A section names the subject when its heading or any of its lines shares one of those words.
- **Endings are allowed, not stems.** Two words are the same when they are equal, or when the shorter has at least four letters, the longer starts with it, and the longer adds at most three letters: `link` and `links`, `Bild` and `Bilder`. No stemmer dependency: the rule only needs to avoid false alarms, and a near-miss keeps it quiet.
- **Long enough means the thin-post length.** A section is reported only when it has at least `thin-post-words` words (300 by default), so it would pass as a post of its own. No new threshold.
- **A warning, never a block.** The writer may have a reason to keep a side topic; the rule teaches the split and leaves the call to them.

## Consequences

- The rule errs towards silence: a long side section that mentions the main subject once is not reported.
- A title with only filler and question words gives no subject, and the rule skips the post.
- The word lists for question words live beside the stopwords in `words.rs` and `german.rs`, so a later shared word home moves them together.
