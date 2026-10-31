# 0000. AI search views name the tool, in the group column

**Number:** next free. **Status:** Proposed, 2026-10-31. GEO M1 (AI visitors per tool), KH-GRO-4.

## Context

The server analytics (ADR "server analytics count on the server") count a view under one of six referrer groups, and every AI assistant shares the group `ai`. M1 asks for AI visitors per tool: how many readers ChatGPT sent, how many Perplexity sent, and so on. The views live in `analytics_counts`, keyed by day, path and `referrer_group`, a text column with no check on its values.

## Decision

- **`ReferrerGroup::Ai` carries an `AiTool`.** The tool comes from the same host list and `utm_source` names the group already used, now paired with the tool each belongs to. A host or source that matched before still matches, so no view moves between groups.
- **The tool is stored in the group column, as `ai:chatgpt`.** No migration and no new column: a view stays one row per day, path and group. Rows stored as plain `ai` before this change read back as `AiTool::Unnamed`, shown as "AI assistant, not named".
- **Only readers are counted per tool, not bots.** The research's "Do not build" list forbids bot counts from the claimed name alone, and an AI bot visit never counts as a citation. Counting AI crawlers per tool waits for the visitor log (C1) and the published address lists (D2).

## Consequences

- A version older than this one cannot read the new rows: it reports `ai:chatgpt` as an unreadable group. Downgrades after views are written need the rows renamed back to `ai`.
- Adding a tool is one line in `ai_tool.rs` and one name; earlier views from it stay under the group they were counted in.
