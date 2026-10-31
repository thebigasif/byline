# 0000. The publishing planner suggests and never writes

**Number:** next free. **Status:** Proposed, 2026-10-31. GEO phase, task M4 (KH-EDT-4, KH-OPS-2).

## Context

GEO task M4 asks for a publishing planner and says nothing more. The GEO source rule allows SEO behaviour only from Steve Toth and Koray Tugberk Gubur. Koray's topical authority work names publication frequency, and keeping it steady, as a signal a site keeps covering its subject. Neither source sets a number of posts a week, a window to compare, or an order of posts inside a topic group. Decision D4 says dates change only on a real update, and the decision of 2026-10-27 says publishing has one door.

## Decision

- `khalid plan` reads every post, the lint publish gate's quick answer and each draft's schedule, and prints a plan. It writes no file and no schedule. A post goes live through `khalid publish` or the admin, the one door.
- The pace is the site's own: its live posts of the last twelve weeks, a week's worth rounded, at least one. `--per-week` from 1 to 14 replaces it. Khalid does not pick a pace for the writer, because neither source gives one.
- Each ready draft gets the next free day at that pace from tomorrow. A day holding a scheduled post is taken.
- A topic group's pillar goes before its members. This is the repository's own cluster rule (ADR "topic cluster front matter"): a member links to its pillar, so the pillar must be live first or the link points at a missing page. Members of a pillar the linter blocks wait with it.
- Warnings: fewer live posts in the last four weeks than in the four before (Koray's steady frequency), a member scheduled before its pillar, and no ready draft. The four-week windows are this ADR's choice, not a source's.
- The logic is pure, in `khalid-seo::plan`, so the admin calendar (KH-EDT-4) and an Admin API route can call it later.

## Consequences

- The planner cannot move a date or publish by itself, so it cannot break D4.
- Steve Toth's work is not used: neither his writing nor Koray's covers a planner's day spacing, so that part is plain arithmetic.
- No Admin API route or admin screen yet; an admin screen needs a mockup and the founder's yes first.
