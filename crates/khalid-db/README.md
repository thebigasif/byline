# khalid-db

The SQLite file at `data/site.db` in a site folder. Content files are the source of truth (ADR 0001). This file holds two kinds of tables:

- **derived**: computed from the files under `content/`. `khalid db rebuild` empties these tables and fills them again from the files. Deleting `data/site.db` loses nothing in them.
- **visitor**: created by people while the site runs, such as comments, subscribers and accounts. A rebuild never touches them, and they never point at a derived row, so a rebuild cannot delete them by cascade. A visitor row that needs content names it by its path.

Every table has a `site_id` that points at `sites`. Schema changes are new files in `migrations/`; an existing migration is never edited (RULES.md BL-DB-2). ADR 0010 explains the layout.

A test reads the table below and checks it lists every table in the schema, once each, as derived or visitor.

| Table | Kind | What it holds |
|---|---|---|
| `sites` | visitor | One row per site the process serves; every other row belongs to one. A rebuild keeps it |
| `authors` | derived | Author profiles from `content/<language>/authors/` |
| `categories` | derived | Categories from `categories.toml` |
| `tags` | derived | Tags named in front matter |
| `posts` | derived | Posts from each collection folder, with their front matter and content hash |
| `post_tags` | derived | Which tags each post carries |
| `pages` | derived | Pages from `content/<language>/pages/` |
| `collection_entries` | derived | Glossary terms, case studies and changelog entries |
| `translations` | derived | Which files are language versions of each other, by their `translation` key |
| `links` | derived | The link graph: every link in every file, with its anchor text and where it resolves |
| `lint_results` | derived | What the linter found in each file |
| `alias_redirects` | derived | Old addresses from `aliases` in front matter, each pointing at its file |
| `redirects` | visitor | Redirects made in the admin, brought in by an import, or added when a slug changes |
| `not_found_log` | visitor | Every address that answered 404, with its hit count and when it was first and last seen |
| `users` | visitor | Staff accounts, with an Argon2id password hash when the account has a password |
| `sessions` | visitor | Signed-in sessions, stored as a hash of the session token |
| `api_keys` | visitor | Integration keys and staff tokens, stored as a hash of the key |
| `api_key_scopes` | visitor | What each key may do |
| `webhooks` | visitor | The addresses that receive each event, with the secret that signs deliveries |
| `webhook_deliveries` | visitor | Each delivery of an event, its attempts and when it is due again |
| `form_submissions` | visitor | What people sent through forms on the site |
| `subscribers` | visitor | Newsletter subscribers and whether they confirmed |
| `comments` | visitor | Comments on posts, named by the post's path so they survive a rebuild |
| `analytics_counts` | visitor | Page views per day, page and referrer group, counted without cookies |

## Changing a query

The `sqlx` macros check every query against the schema when the crate compiles (RULES.md BL-DB-3). CI has no database, so it reads the checked query shapes from `.sqlx/` at the repository root. After changing a query or adding a migration, run `just db-prepare` and commit what changes in `.sqlx/`.
