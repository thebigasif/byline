# 0030. Drafts live in `drafts/` until they are published

**Status:** Accepted, 2026-10-26. Phase 1 tasks 1.15 and 1.18, the API wiring.

## Context

The Admin API creates posts as drafts, and SerpDad stages every finished article as one (plan section 8). A draft must never be served, listed, put in the sitemap or counted by the site's lint rules. Every file under `content/<language>/posts/` is rendered and served today, and post front matter has no draft or status field. The content model belongs to the content track, so this track cannot add one.

## Decision

- A draft is a markdown file at `drafts/<language>/<slug>.md` in the site folder, with the same front matter as a post: a quoted title, the slug and the author. Nothing reads `drafts/` but the Admin API.
- Publishing adds `date:` with today's date, runs the lint publish gate (`Linter::gate`) on the post as it would go live, and only when the gate lets it through, writes `content/<language>/posts/<date>-<slug>.md` atomically and removes the draft. A refused draft stays where it was, unchanged.
- A post's identity is `<language>.<slug>`, the same for its draft and its published file, in both APIs.
- An edit changes only the front matter fields it names, line by line, so every other line stays as the writer left it.
- A post last changed when its file did. An edit made by hand in an editor therefore also makes an older API copy stale (BL-API-7).

## Consequences

- Drafts are files, like everything else a writer makes (ADR 0001). They go into backups and git with the rest of the site.
- `content/` keeps meaning "what readers see". No rule, template or feed needs a draft check.
- When the content track adds a status field, drafts can move under `content/` in one change to the post store, with no change to either API.
- Git mode (plan section 5) can later make each draft a branch without changing the API.
