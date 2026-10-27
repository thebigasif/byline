# 0000. The SEO score sums up a post's findings with fixed weights

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 1, audit brief B-LNT-4 (KH-LNT-4, plan D-10).

## Context

KH-LNT-4 asks for an SEO score out of 100 per post "that summarises the checks and never replaces them", and plan D-10 adds that the score points at the explanations. Rank Math gives each of its tests a hidden weight and adds up the tests a post passes. Our rules only report problems, and a site can switch any rule off or turn a warning into a block in `lint.toml`, so a score has to come from the findings and the severities the site chose.

## Decision

`Report::score(post)` returns a `Score`: a value from 0 to 100 and the list of rules that cost points, in the order of their first finding.

- Start at 100. Each rule that found a problem in the post costs points once: 30 for a block, 5 for a warning. A rule that fires on twenty images costs as much as one that fires once, because one lesson fixes all of them.
- A post with any block scores at most 40, so a post the publish gate refuses never looks nearly done.
- The score never goes below 0. A rule set to `off` never runs, so it costs nothing.
- The score serialises as `{"value": 85, "lost": [{"rule": "missing-alt", "points": 5}]}`, so the dashboard shows the number and links each lost point to its finding and message.

## Consequences

- The weights are the same for every rule, so the score is easy to explain in one sentence and changes the same way whichever rule a writer fixes.
- A site that turns a warning into a block in `lint.toml` sees that rule weigh more, which matches what the site said matters.
- A post without a focus keyword gets no focus findings, so it can score 100. Whether that should cost points is a question for the founder in the phase brief.
- Showing the score in `khalid lint` output and in the admin is wiring in the binary and in Phase 4; the library gives both the same numbers.
