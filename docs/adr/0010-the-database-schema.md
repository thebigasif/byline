# 0010. The database: derived tables, visitor tables, one writer

**Status:** Accepted, 2026-10-26. Phase 1 task 1.10. ADR 0001 and 0002 decide that there is one SQLite file per site under `data/`; this record decides its shape.

## Context

ADR 0001 says deleting `data/site.db` loses visitor data and nothing else. That only holds if the schema keeps the two kinds of data apart: a rebuild must be able to empty everything computed from the files without touching a single comment or subscriber. SQLite also allows one writer at a time, and the server must never stall a page read behind a write.

## Decision

- **Two kinds of table, listed in `crates/khalid-db/README.md`.** Each table is either derived (rebuilt from `content/`) or visitor (created by people at run time and kept). A test fails if the README and the schema disagree.
- **Visitor tables never hold a foreign key to a derived table.** A comment names its post by the post's path, not by a row id, so emptying `posts` cannot cascade into `comments`, and a comment finds its post again after a rebuild.
- **Redirects are two tables.** `alias_redirects` comes from `aliases` in front matter and is derived. `redirects` holds what people create in the admin, what an import brings, and the automatic redirect when a slug changes, and is visitor data.
- **Every table has `site_id`**, pointing at `sites`, which a rebuild keeps. One file per site is the default, and the column lets one process serve several sites from one file later without a migration that touches every table.
- **Tables are `STRICT`**, so SQLite refuses a value of the wrong type instead of storing it.
- **One writer connection and a read pool of eight.** All writes go through the writer, so they queue in Khalid instead of failing on a lock. Readers are opened read-only, so a bug cannot write through one. WAL mode lets readers run during a write. `synchronous = NORMAL` is safe in WAL mode and keeps commits fast. The busy timeout is five seconds.
- **Author and category on a post are slugs, not foreign keys.** A post that names a missing author is a content mistake the linter explains; the database must still index it.

## Consequences

- A rebuild is one transaction: delete every derived row for the site, insert the index again. Readers see the old index or the new one, never half of each.
- Adding a table means adding its line to the README, as derived or visitor.
- A visitor table that needs content uses the content's path, which is stable across rebuilds and changes only when the file moves.
