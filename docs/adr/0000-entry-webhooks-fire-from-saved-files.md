# 0000. Page and collection entry webhooks fire from saved files

**Number:** next free. **Status:** Proposed, 2026-10-31. Audit brief G25, KH-API-3.

## Context

The event catalogue (ADR 0014) has five events each for pages, glossary terms, case studies and changelog entries. Brief G25 asks for them "from the door". Posts have a door: the publish door in `khalid-api` fires `post.*`. Pages and the other entries have none on `main`: the G24 entry routes are still open pull requests, and these entries have no draft folder, no status and no publish time. On `main` they change only when someone saves their file, and `khalid serve` sees every save through its watch.

## Decision

- **`khalid serve` compares the entries of each full render with the last.** Each render's catalog for the Content API (`crates/khalid/src/site_content.rs`) carries the entries and their file hashes. The Content API's task compares them before it builds the store (`crates/khalid/src/entry_events.rs`). The first catalog is compared with the content the run started with, so a save during the start is not lost.
- **A file that is there is live.** A new file fires `<kind>.published`, a changed hash `<kind>.published.edited`, and a removed file `<kind>.deleted` with the entry as it was. Renaming a file is a delete and a publish.
- **An entry moved to `drafts/` is unpublished, not deleted.** Posts already wait in `drafts/<language>/` (ADR 0030). An entry waits at the same place under `drafts/` as it had under `content/`: `drafts/en/pages/about.md` for `content/en/pages/about.md`. When a removed file has such a draft in the same render, it fires `<kind>.unpublished` instead of `<kind>.deleted`. Moving it back fires `<kind>.published`. The post list reads only the files directly under `drafts/<language>/`, so these folders never show up as posts.
- **The body names the entry by kind:** `page`, `glossary_term`, `case_study` or `changelog_entry`, each with `id` (`<language>.<slug>`), `language`, `slug`, `title` and `file`. The slug is the front matter's, else the file name.
- **Posts stay with the door.** Post files are left out of the comparison, so a post event never fires twice.

## Consequences

- `<kind>.scheduled` does not fire for these entries yet: nothing gives one a publish time.
- When the G24 entry door lands, it writes files, so the same comparison fires its events. The door itself should not fire them, or each would arrive twice. To take an entry down without deleting it, the door moves its file to `drafts/`.
- A move needs both sides in one render. A rename within one disk is one step, so it always is; a copy to `drafts/` made well after the delete fires `.deleted`.
- A save made while `khalid serve` is stopped fires nothing on the next start.
- Saves close together are rendered once, so a file added and changed before the render fires only `.published`.
