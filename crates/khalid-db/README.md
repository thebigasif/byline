# khalid-db

The SQLite file at `data/site.db` in a site folder. Content files are the source of truth (ADR 0001). This file holds two kinds of tables:

- **derived**: computed from the files under `content/`. `khalid db rebuild` empties these tables and fills them again from the files. Deleting `data/site.db` loses nothing in them.
- **visitor**: created by people while the site runs, such as comments, subscribers and accounts. A rebuild never touches them.

Every table has a `site_id` that points at `sites`. Schema changes are new files in `migrations/`; an existing migration is never edited (RULES.md BL-DB-2).

| Table | Kind | What it holds |
|---|---|---|
| `sites` | visitor | One row per site the process serves; every other row belongs to one. A rebuild keeps it |
