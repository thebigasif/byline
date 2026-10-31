# API conventions

The API is the only door (CLAUDE.md). This file fixes the shape so every endpoint looks like every other one.

## Surfaces

| Surface | Base path | Auth | Purpose |
|---|---|---|---|
| Content API | `/api/content/v1/` | Content key in `Authorization: Khalid-Content <key>` or `?key=`, safe in browsers | Read public content |
| Admin API | `/api/admin/v1/` | Integration key or staff token in `Authorization: Khalid <token>`, or the admin session cookie plus CSRF | Read and write everything |
| oEmbed | `/api/oembed` | None | Embed cards for other sites |
| Publish target | Part of the Admin API | Integration key with the `posts:write` scope, and `posts:publish` to publish | SerpDad and other tools stage drafts |

## Shape

- Resources are plural nouns: `posts`, `pages`, `authors`, `tags`, `categories`, `collections`, `glossary`, `case-studies`, `changelog`, `media`, `redirects`, `snippets`, `forms`, `subscribers`, `webhooks`, `settings`, `site`.
- A response wraps the resource in an array under its name and adds `meta`: `{"posts": [...], "meta": {"pagination": {"page": 1, "limit": 15, "pages": 4, "total": 52, "next": 2, "prev": null}}}`.
- Lists accept `filter`, `include`, `fields`, `formats`, `limit`, `page`, `order`. The `filter` grammar is Ghost's: `tag:news+featured:true`, `tag:[seo,links]`, `tag:-draft`, `published_at:>'2026-01-01'`. One crate parses it, with its own tests (BL-API-5).
- `formats` on posts selects `html`, `markdown`, `plaintext`, any combination.
- Writes accept and return the same shape. A `POST /posts/?source=html` accepts an HTML body and converts it to markdown and blocks.
- Edits send `updated_at`. A stale value returns 409 with a problem detail that names who changed it and when (BL-API-7).
- Errors are RFC 9457: `{"type": "https://khalid.dev/problems/lint-blocked", "title": "The post cannot be published", "status": 422, "detail": "This post has no author.", "fix": "Add author: to the header.", "rule": "no-author", "instance": "/api/admin/v1/posts/abc"}`.
- `Accept-Version: v1.3` sets the minimum version the client expects. The server answers with the version it served.
- Every endpoint is in the OpenAPI document at `/api/openapi.json`, generated from `utoipa` annotations, and a contract test fails when the document and the routes disagree (BL-API-4).

## Keys and scopes

Integration keys are created in the admin or with `khalid keys create`. Each key has scopes: `content:read`, `posts:write`, `posts:publish`, `media:write`, `settings:write`, `webhooks:manage`, `subscribers:write`, `admin:all`. Keys are stored hashed and shown once. Staff tokens carry the user's role. For example, `khalid --site <folder> keys create --name SerpDad --scope posts:write` makes an integration key that writes drafts, `--scope posts:write --scope posts:publish` one that also publishes them, `--kind content` a content key and `--kind staff --role editor` a staff token, and `khalid keys revoke <id>` stops one at once.

## Webhooks

Events are `resource.verb`: `post.published`, `post.published.edited`, `post.unpublished`, `post.scheduled`, `post.deleted`, the same for `page` and each collection, `tag.added`, `form.submitted`, `subscriber.added`, `comment.added`, `lint.blocked`, `audit.finished`, `site.changed`, `alert.raised`. The body is the resource in API shape plus `event`, `occurred_at` and `site`. Deliveries are signed: `Khalid-Signature: t=<unix>,v1=<hmac-sha256(secret, t + "." + body)>`. Retries back off over 24 hours. The admin shows a delivery log.

Pages, glossary terms, case studies and changelog entries fire their `.published`, `.published.edited` and `.deleted` events from `khalid serve` as their files are added, changed or removed. The body names the entry by kind, for example `{"page": {"id": "en.about", "language": "en", "slug": "about", "title": "About Field Notes", "file": "content/en/pages/about.md"}}`. An entry is taken down by moving its file to the same place under `drafts/`, such as `drafts/en/pages/about.md`, which fires `.unpublished` with the entry as it was; moving it back fires `.published`. They have no publish time yet, so their `.scheduled` events wait for the entry routes. `site.changed` fires when `khalid serve` sees a saved `site.toml` whose settings differ from the last good one, with `{"settings": {"file": "site.toml", "changed": ["name", "seo"], "restart_needed": true}}`: the top-level settings and tables that changed, by name only, since a value may be private. A file that is not valid TOML fires nothing until it is fixed. A tag is made by naming it on a post, so `tag.added` fires when a live post first names a tag no other post in that language had, with the tag in the Content API's shape: `{"tag": {"id": "en.crawl-budget", "slug": "crawl-budget", "name": "crawl budget", "language": "en"}}`. `subscriber.added` fires when a reader opens their confirmation link, not when they fill in the form, with `{"subscriber": {"id": 12, "confirmed_at": "2026-10-31T09:30:00Z"}}`. The address stays out of the body, as a commenter's does. `audit.finished` fires as `khalid audit` ends, with `{"audit": {"score": 100, "pages": 48, "errors": 0, "warnings": 22, "notices": 3}}`; the issues stay in `data/audit/`.

A webhook sends one event to one address. A key with `webhooks:manage` lists them with `GET /api/admin/v1/webhooks/`, adds one with `POST /api/admin/v1/webhooks/` and a body such as `{"webhooks": [{"event": "comment.added", "url": "https://hooks.example.com/khalid"}]}`, and switches one off with `DELETE /api/admin/v1/webhooks/{id}/`. The answer that adds a webhook shows its secret once; the receiver checks each signature with it. On the command line, `khalid --site <folder> webhooks add --event comment.added --url https://hooks.example.com/khalid` does the same, `webhooks list` lists them and `webhooks remove <id>` switches one off. A webhook added while the site runs is sent to and retried at once.

## Versioning

Additive changes stay in `v1`: new fields, new endpoints, new filters. Removing or renaming anything is `v2`, announced one minor version ahead in the changelog and in a response header.
