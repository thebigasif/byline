# 0000. A post names its focus keyword and up to four secondary keywords

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 1, audit brief B-LNT-4 (KH-LNT-4).

## Context

KH-LNT-4 asks for a focus topic per post: one primary keyword and related terms, checked by the linter and summed up in a score. The audit brief suggests the field names `focus`, `keyword` and `related`. Two of those clash with what exists: `related` already pins the posts shown under a post (plan section 4.12), and `focus` alone does not say what kind of value it holds. Writers who move from WordPress know the words "focus keyword" from Yoast and Rank Math, and the WordPress import reads Rank Math's `rank_math_focus_keyword` as a list with the main keyword first.

## Decision

Post front matter gains two optional fields:

- `focus_keyword`: one phrase, such as `internal links`. Blank text is refused with a message that says to remove the field instead.
- `secondary_keywords`: one phrase or a list, at most four, so five keywords in all, the number Rank Math's free version allows. Each must hold a word.

The lint model names its field `focus_keyword` too, and gains `secondary_keywords`, so the header, the lint input and the messages use the same words. The WordPress import can then write a Rank Math list as its first keyword in `focus_keyword` and the rest in `secondary_keywords`; that mapping belongs to the import track.

## Consequences

- The field names read the same in a post, in a lint message and in the import, so a writer meets one word for one idea.
- A post with more than four secondary keywords is refused with the count, so the writer chooses which matter. Rank Math's paid tiers allow more; we can raise the limit later without breaking a file.
- Both fields are optional. A post without a focus keyword gets no focus checks.
