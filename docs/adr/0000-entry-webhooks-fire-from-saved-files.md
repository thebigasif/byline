# 0000. Page and collection entry webhooks fire from saved files

**Number:** next free. **Status:** Proposed, 2026-10-31. Audit brief G25, KH-API-3.

## Context

The event catalogue (ADR 0014) has five events each for pages, glossary terms, case studies and changelog entries. Brief G25 asks for them "from the door". Posts have a door: the publish door in `khalid-api` fires `post.*`. Pages and the other entries have none on `main`: the G24 entry routes are still open pull requests, and these entries have no draft folder, no status and no publish time. On `main` they change only when someone saves their file, and `khalid serve` sees every save through its watch.

## Decision

- **`khalid serve` compares the entries of each full render with the last.** Each render's catalog for the Content API (`crates/khalid/src/site_content.rs`) carries the entries and their file hashes. The Content API's task compares them before it builds the store (`crates/khalid/src/entry_events.rs`). The first catalog is compared with the content the run started with, so a save during the start is not lost.
- **A file that is there is live.** A new file fires `<kind>.published`, a changed hash `<kind>.published.edited`, and a removed file `<kind>.deleted` with the entry as it was. Renaming a file is a delete and a publish.
- **The body names the entry by kind:** `page`, `glossary_term`, `case_study` or `changelog_entry`, each with `id` (`<language>.<slug>`), `language`, `slug`, `title` and `file`. The slug is the front matter's, else the file name.
- **Posts stay with the door.** Post files are left out of the comparison, so a post event never fires twice.

## Consequences

- `<kind>.unpublished` and `<kind>.scheduled` do not fire for these entries yet: nothing can take one down without deleting it, or give it a publish time. They fire once the entry door (G24) adds those states.
- When the G24 entry door lands, it writes files, so the same comparison fires its events. The door itself should not fire them, or each would arrive twice.
- A save made while `khalid serve` is stopped fires nothing on the next start.
- Saves close together are rendered once, so a file added and changed before the render fires only `.published`.
