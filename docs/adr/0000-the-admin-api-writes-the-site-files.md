# 0000. The Admin API writes the site's own files, and a stale copy is refused

**Number:** next free. **Status:** Accepted, 2026-10-31. Audit 2, brief G24 (KH-API-2, BL-API-7, BL-FS-1).

## Context

The brief asks for Admin API read and write routes for pages, authors, tags, categories, site settings, snippets and keys, each with success, validation, auth and stale-edit tests and an OpenAPI entry. Content files are the source of truth (BL-FS-1), and settings are read once at start. The plan does not say where each of these lives behind the API, which scopes guard them, or how a stale edit is detected for things that are not one file each.

## Decision

- **Pages, snippets and authors are one store.** Each is one markdown file per language under `content/<language>/<folder>/`. `khalid-api`'s generic `EntryStore` keeps them; `khalid serve` reads and writes the files through `SiteEntries`, editing only the front matter lines a change names. The identity is `<language>.<slug>`.
- **A tag lives in its posts.** It has no file and no create route. It is listed, renamed or removed in every post that carries it, drafts included, by rewriting each post's `tags` line. A rename onto an existing name merges the two tags.
- **Categories stay in `categories.toml`,** edited in place with `toml_edit` so its comments stay. The identity is the slug. A category posts are still in is never deleted (409).
- **Core settings stay in `site.toml`,** edited in place and checked as at start before anything is written. A saved change applies after a restart, and the answer says when one is needed. Mode and slug style are left out, because changing them under a running site moves its addresses.
- **Keys are listed, made and revoked** in the same key store every request is checked against. A key is shown once and only its hash is stored. Staff tokens are not made here. The key making a request cannot revoke itself.
- **A stale copy is refused with a 409.** `updated_at` is the file's modified time: per file for pages, snippets and authors, the newest of a tag's posts for a tag, and the whole file for categories and settings. A hand edit therefore makes an older copy stale too.
- **Scopes.** Reading pages, snippets, authors, tags and categories needs `posts:write`. Changing them needs `posts:publish`, because each change shows on live pages at once. Settings need `settings:write`; keys need `admin:all`.

## Consequences

- An edit through the API and an edit in a text editor meet the same stale check, so neither silently undoes the other.
- Tag and category changes bypass the publish door: they do not change whether a post may be live, so they run no lint gate and fire no `post.published.edited`. `tag.added` and a rename event are left to the webhooks follow-up.
- A file's modified time has the coarse clock of the file system, so two writes a few milliseconds apart can share one `updated_at`. Posts have the same limit.
- The SEO tables in `site.toml` (brief G11) and staff tokens have no Admin API routes yet.
