# khalid-db

The SQLite file at `data/site.db` in a site folder. Content files are the source of truth (ADR 0001). This file holds two kinds of tables:

- **derived**: computed from the files under `content/`. `khalid db rebuild` empties these tables and fills them again from the files, and after a save `khalid serve` writes again only the rows of the posts that changed (`Database::update`). Deleting `data/site.db` loses nothing in them.
- **visitor**: created by people while the site runs, such as comments, subscribers and accounts. A rebuild never touches them, and they never point at a derived row, so a rebuild cannot delete them by cascade. A visitor row that needs content names it by its path.

Every table has a `site_id` that points at `sites`. Schema changes are new files in `migrations/`; an existing migration is never edited (RULES.md BL-DB-2). ADR 0010 explains the layout.

`SiteExport` reads one site's rows out of every table for `khalid db export`. Sign-in tables (`sessions`, `magic_links`, `passkeys`, `subscriber_tokens`, `site_keys`) are left out, and so is any column named like a secret: `secret`, `token`, `password`, or ending in `_hash` other than `content_hash`. Name a new secret column that way.

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
| `search_pages` | derived | Each published page site search can find: its address, title, plain-text body and words per field |
| `search_terms` | derived | The search index: each term, the pages that hold it, and how often in the title, headings and body |
| `redirects` | visitor | Redirects made in the admin, brought in by an import, or added when a slug changes |
| `not_found_log` | visitor | Every address that answered 404, with its hit count and when it was first and last seen |
| `users` | visitor | Staff accounts, with an Argon2id password hash when the account has a password |
| `sessions` | visitor | Signed-in sessions, stored as a hash of the session token, with when each was last used |
| `passkeys` | visitor | Each staff passkey: the authenticator's name for it, its public key and its signature counter |
| `magic_links` | visitor | Sign-in links sent by email, stored as a hash of the token, each usable once before it expires |
| `api_keys` | visitor | Content keys, integration keys and staff tokens: each key's identity, kind, staff role and a hash of its secret, never the secret |
| `api_key_scopes` | visitor | The scopes of each integration key |
| `webhooks` | visitor | The addresses that receive each event, with the secret that signs deliveries |
| `webhook_deliveries` | visitor | Each delivery of an event: the bytes sent, where it stands and when it is due again |
| `webhook_attempts` | visitor | Every attempt at a delivery, with the status that came back or why there was none |
| `post_schedules` | visitor | When each post goes live and when it comes down, named by its identity so a rebuild keeps it |
| `site_keys` | visitor | Keys the site makes for itself, such as the one that signs preview links; 256 random bits each, never shown |
| `form_submissions` | visitor | What people sent through forms on the site |
| `subscribers` | visitor | Newsletter subscribers, whether they confirmed, and when they left |
| `subscriber_lists` | visitor | Which newsletters each subscriber gets, by the newsletter's name |
| `subscriber_labels` | visitor | The labels each subscriber carries, such as paid, which make segments |
| `subscriber_tokens` | visitor | The confirmation and unsubscribe links sent to each subscriber, as hashes of their tokens |
| `newsletter_sends` | visitor | Each email that went out to a newsletter, under a key that stops a post going out twice, with its totals: copies sent, refused and unsubscribes from it |
| `comments` | visitor | Comments on posts, named by the post's path so they survive a rebuild: who wrote each, a reader or staff, its text and where it stands in moderation |
| `analytics_counts` | visitor | Page views per day, page and referrer group, counted without cookies |
| `crawl_log` | visitor | Which bots asked for which address each day, as their user agent named them, with the latest time and status; 90 days kept |
| `visitor_log` | visitor | One row per visit while `[visitor_log]` is on: time, address asked for, status, shortened network address, a visitor code that changes daily, and the bot's name and address check or the referrer group; 7 to 14 days kept |
| `crawled_pages` | visitor | Each page a bot fetched from an address on its company's list, with the first and last fetch; written with the visitor log and kept when its rows are erased |
| `alerts_raised` | visitor | The key of each alert finding sent to the owner, so it is sent once while the problem lasts |
| `audit_log` | visitor | Each change made through the Admin API: who made it, whether it added, changed or removed something, the route and the address; the newest 100,000 kept |
| `site_watch` | visitor | Each look the speed and uptime watch took at a page of the public address, with its status and time; 30 days kept |

## Changing a query

The `sqlx` macros check every query against the schema when the crate compiles (RULES.md BL-DB-3). CI has no database, so it reads the checked query shapes from `.sqlx/` at the repository root. After changing a query or adding a migration, run `just db-prepare` and commit what changes in `.sqlx/`.
