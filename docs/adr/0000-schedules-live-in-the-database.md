# 0000. A post's schedule lives in the database, beside its file

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 5 work pulled forward (CTO answers 2026-10-27, section 4).

## Context

The publish door takes `published_at` with `status: scheduled`, and `unpublish_at`, on an Admin API edit. The time has to be kept somewhere until the scheduler in `khalid serve` acts on it, including across a restart. Pull request #406 offered two places: `publish_at:` and `unpublish_at:` in post front matter, or one `schedule.toml` at the top of the site folder. The decision of 2026-10-27 did not choose. `PostFront` belongs to the content track and refuses fields it does not know, and plan section 4 lists "scheduled jobs" among what the database holds.

## Decision

- **Schedules are a visitor table, `post_schedules`, in `data/site.db`.** One row per post with either time set, named by the post's identity, `<language>.<slug>`, so a rebuild of the index keeps it. `khalid_db::ScheduleTable` implements `khalid_api::schedule_store::ScheduleStore`.
- **The post file is not touched by scheduling.** A scheduled post waits in `drafts/` (ADR 0030) until the scheduler publishes it through the door, which dates it and moves it into `content/` as any publish does.
- **Times are UTC to the second**, written by Khalid's clock as RFC 3339 text, so they sort as text and the due query is one indexed scan per site.

## Consequences

- A lost database loses the schedule, never the post: every scheduled post stays a draft, which is the safe way to fail. The Admin API shows it as a draft again, and it can be scheduled once more.
- A writer reading a draft file does not see its publish time in the file. The admin and the API show it. If the founder wants the time in the file, the content track can add `publish_at:` to `PostFront` later and this table becomes a cache of it; nothing in the door changes.
- Editing a draft by hand while it is scheduled keeps the schedule, since the row is named by identity, not by file contents. Renaming the draft's slug by hand drops it.
