# 0000. A site's facts live in facts.toml, and the linter holds posts to them

**Number:** next free. **Status:** Proposed, 2026-10-31. GEO phase, task P7, the fact registry (KH-LNT-2, KH-LNT-3, KH-LNT-6). Taken under the founder's yes in advance of 2026-10-27; the phase brief gives P7 no done line, so this record sets the scope.

## Context

P7 asks for a fact registry. Posts on one site repeat the same facts: a price, a founding year, a team size, a limit. When one changes, older posts keep the old value, and the site then says two different things about one fact. Koray Tugberk Gubur teaches that a source must keep its declarations consistent, because contradicting itself costs it trust. Decision D4 asks for a review every six months and says dates change only on a real update.

Neither Steve Toth nor Koray Tugberk Gubur says how a registry file is laid out, how a value is matched in a post, or how long a fact may go unchecked. Those details are our own calls, listed below.

## Decision

- A site keeps its facts in `facts.toml` beside `lint.toml`. Each table is one fact under a key of lowercase letters, digits and hyphens. It holds `value`, the value posts should give; `old`, the values it held before; and `checked`, the plain date someone last made sure the value holds. TOML refuses a key used twice, so a fact has one value by construction.
- The file is person-written, so it lives at the site root with the other settings, not in `content/` and not in `data/`.
- The binary reads it once per lint, build or publish, and refuses a broken file with what, why and fix. A missing file means no facts.
- The linter gets the facts as `LintFact` through `SiteView::facts`, so SerpDad's backend can pass its own. Two rules read them:
  - `fact-old-value` warns when a post's title, description or body gives an old value. The finding names the old value and suggests the current one. An old value that is part of the current one is never flagged.
  - `fact-not-checked` warns on each post that gives the value of a fact whose `checked` date is older than `fact-check-months` in `lint.toml`, default 6, the D4 cadence.
- Values match as whole words, ignoring letter case. Other ways of writing the same number, such as "19 dollars", are not matched.
- Both rules warn. A site that wants a hard stop sets either to `block`, and the build and the publish door then refuse.

## Consequences

- Changing a fact is one edit to `facts.toml`, and `khalid lint` then lists every post that still gives the old value.
- Posts do not pull a value from the registry yet: a writer still types it. A placeholder that renders the current value needs the content and render crates, which another track owns. It is the natural next step.
- There is no `source` field. W4 asks posts to link a number's source; a registry field would only help once posts can pull values in.
- The demo site has no `facts.toml`, so its lint and audit results do not change.
