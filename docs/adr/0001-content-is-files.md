# 0001. Content is files, the database is for the rest

**Status:** Accepted, 2026-10-26. From the founder's plan, section 4.2.

## Context

A blog engine needs a web editor, search, comments, a newsletter and analytics, which all need somewhere to write. It also needs to work for agents and command-line users, survive its own death, and migrate honestly. WordPress puts everything in a database that only the editor can read. Static site generators put everything in files and then cannot run search or comments.

## Decision

Every post, page, author profile, glossary entry, case study, changelog entry and snippet is a markdown file with typed front matter under `content/`. The web editor edits those files. One SQLite file per site under `data/` holds what is derived from the files (the index, the link graph, lint results, the search index, rendered caches) and what visitors create (comments, subscribers, form submissions, analytics counts, sessions). Deleting `data/site.db` loses visitor data and nothing else; `byline serve` rebuilds the rest.

## Consequences

- Migration is a folder anyone can read. Backup is a copy. History is git.
- Nothing derived may be written under `content/` (BL-FS-2). Writes to `content/` are atomic (BL-RUN-9).
- Two writers can race on a file. The admin uses presence and locks, and the API uses `updated_at`.
- The file tree must rebuild incrementally; a full rebuild of 10,000 posts stays under 30 seconds (BL-FS-4).
