# Phase 1: the engine and the API

**Goal:** the demo site served locally from markdown files, scoring 100 on Lighthouse, and the same posts fetched through the Content API. Plan section 10, Phase 1. Three weeks of agent work.

**Shows the founder:** `khalid serve --site fixtures/demo-site` serves the five demo posts in English and German as semantic HTML that works with scripts off, and `curl /api/content/v1/posts/` returns the same posts as JSON.

**Inputs that are decided:** everything in `docs/plan.md` sections 1 and 12, and `docs/decisions/2026-10-26-cto-answers.md`. The product is Khalid. Slugs are ASCII by default with per-language transliteration and a per-site Unicode opt-in.

**How each task lands:** the flow in `CLAUDE.md`. One task, one landing, under 400 changed lines; split anything bigger into parts that each land green. Each task updates its status line here when it lands.

## Foundations

### 1.1 Settings from `site.toml`

- Status: `done` (#23, #24 and this landing). `khalid --site <folder>` checks the settings first; `fixtures/invalid/` holds one folder per mistake with its exact message.
- `Config` reads `site.toml`, the environment and command-line flags once at start, in that order of precedence, into typed fields: address (`AbsoluteUrl`), mount path, languages (`Locale`), default language, mode (server or static), slug mode (ascii or unicode). The config module carries the one `#[expect(clippy::disallowed_methods)]` for reading the environment, which closes task 0.4's config part.
- Rules: BL-RUN-2, BL-FS-3, BL-ERR-4, BL-TYP-3.
- Done when: `fixtures/demo-site/site.toml` loads, and a file with a wrong field produces a message that names the file, the line and the field, in what, why and fix.

### 1.2 `Locale`, `AbsoluteUrl` and `SitePath`

- Status: `done` (#21, #22). Built before 1.1, because settings need these types.
- The remaining identifier newtypes from `docs/standards/rust.md`, each with a fallible constructor. `SitePath::within(root, untrusted)` cannot escape the site folder and carries the one `Path::join` `#[expect]`.
- Rules: BL-TYP-3, BL-SEC-4.
- Done when: property tests show no input escapes the root (including `..`, absolute paths, symlinks and encoded separators), and the suppression gate in `just check` still passes.

### 1.3 Slugs from titles

- Status: `done` after this landing.
- `Slug::from_title(title, locale)`: in German, `über` becomes `ueber` and `Straße` becomes `strasse`, other Latin scripts lose their diacritics, other scripts take the Unicode path. With `slugs = "unicode"` a slug is NFC-normalised lowercase letters, digits and hyphens from any script, and the 200-byte limit applies to the percent-encoded form. `Slug::try_new` stays the strict ASCII constructor.
- Rules: BL-TYP-3. CTO answers, section 2.
- Done when: fixture tests cover German, French, Polish, Arabic and Japanese titles in both modes.

### 1.4 The outbound client

- Status: `done` (#28, #29, #30 and this landing): the address guard, the guarded resolver, the client, and redirects through the guard.
- `crates/khalid-outbound`: the one HTTP client, with timeouts, a response size cap, a redirect cap, and a guard that refuses private, loopback and link-local addresses after DNS resolution. Its module carries the one `disallowed_types` `#[expect]` for `reqwest::Client`, and `lints/check` gains the `reqwest::Client` violation, which closes task 0.4.
- Rules: BL-RUN-8, BL-TST-3.
- Done when: `wiremock` tests cover each cap and each refused address range, and `scripts/check-lints.sh` no longer prints "Not proven yet".

## Content

### 1.5 Front matter and the content tree

- Status: `done` (#33 to #37 and this landing). KH-CON-1 to KH-CON-4.
- `crates/khalid-content`: typed front matter for posts, pages, authors, glossary terms, case studies, changelog entries and snippets, with errors that name the file, line and field. The content tree reads language folders, post folders with co-located assets, and the `translation` key that pairs language versions.
- Rules: BL-FS-1, BL-FS-3, BL-TYP-3.
- Done when: a test asserts `fixtures/demo-site` parses into five English posts, five German posts, two authors per language and one glossary term per language, which also closes task 0.5. Each invalid fixture produces its plain message.

### 1.6 Markdown and the directive layer

- Status: `doing`. (a) #50: `render()` with comrak, escaped raw HTML, heading ids, links, images and word count. (b) this landing: syntax colours with syntect, as `hl-` CSS classes. (c) the Block model with parse, print and the round-trip harness. (d) directives, callout and figure, ADR 0009.
- `comrak` with footnotes, tables and task lists, plus the directive syntax every editor block uses. The round-trip harness: markdown to blocks to markdown, byte for byte. First blocks: heading, paragraph, image with required alt and caption, quote, code with syntax colours, divider, table, footnote, callout.
- Rules: BL-FS-5, BL-TST-5, BL-SEC-1.
- Done when: every listed block has a directive, a round-trip test and an HTML snapshot, and raw HTML in markdown is escaped.

### 1.7 The remaining blocks

- Status: `todo`
- The rest of plan section 4.6's block list, in parts of about five blocks each: gallery, video, audio, file download, bookmark card, product card, toggle, button, header, embed, HTML with visibility rules, FAQ, how-to, review, recipe, business card, form, newsletter signup, gated content, table of contents. Blocks that need a server feature (form, newsletter, gated content) render their plain HTML form and lock card for now.
- Rules: BL-FS-5, BL-TST-5, BL-SEC-1.
- Done when: every block in section 4.6 has a directive, a round-trip test and a snapshot.

### 1.8 Wikilinks and maths

- Status: `todo`
- `[[slug]]` resolves to posts, pages and glossary terms at build, in the same language, and reports broken ones. Maths renders on the server as MathML with `latex2mathml`.
- Rules: BL-PERF-2.
- Done when: the demo site's two glossary wikilinks resolve in both languages, a broken one is reported with file and line, and a maths fixture renders to MathML with no JavaScript.

### 1.9 Routes, collections and channels

- Status: `todo`
- `routes.toml` per plan section 4.11: collections with URL patterns, channels, taxonomy prefixes, aliases, mount path and language prefix. Defaults need no file. The glossary, case studies and changelog collections from section 5.13 with their typed front matter.
- Rules: BL-FS-3. Plan sections 4.11 and 5.13.
- Done when: URL tests cover the default `/blog/{slug}/`, a `/news/{year}/{slug}/` collection, `/de/` prefixes, a mount path, and an alias that becomes a redirect.

### 1.10 The database

- Status: `doing`. Part 1 (#39): `crates/khalid-db`, opening the file with WAL, foreign keys, a busy timeout, one writer and a read pool, and forward-only migrations. Part 2 (this landing): the derived tables of the content index, ADR 0010, and a test that the crate README lists every table as derived or visitor. Part 3: the visitor tables. Part 4: the rebuild and the list queries with their query plans. Part 5: `khalid db migrate` and `khalid db rebuild`.
- `crates/khalid-db`: SQLite in WAL mode with foreign keys, a busy timeout, one writer and a read pool; forward-only migrations; the index rebuilt from content files on demand. Nothing derived is written under `content/`.
- Rules: BL-DB-1 to BL-DB-3, BL-FS-1, BL-FS-2, BL-RUN-3.
- Done when: deleting `data/site.db` and starting again rebuilds the same index, checked by a test.

## Output

### 1.11 Templates

- Status: `doing`. Part 1 (#43): `crates/khalid-render` and its input records. Part 2 (#45): strings per language, text direction, HTML escaping. Part 3 (#46): the renderer, the base layout with the head and share card, and the 404 page. Part 4 (#47): the snapshot and HTML validity checker every page type passes. Part 5 (#48): templates in `theme/templates/` replace the defaults, with the sandbox tests. Part 6 (#52): the post page, with dates in the page's language. Part 7 (#57): responsive images as `picture` with AVIF and WebP, and the post's lead image. Part 8 (this landing): the home page and collection lists with pagination. Next: the strings, the renderer and its sandbox, theme replacements, then each page type.
- `crates/khalid-render`: sandboxed MiniJinja with autoescape, default templates for home, post, page, author, tag, category, collection index, glossary A to Z and 404, semantic HTML with one `h1`, `lang` and `dir`, strings in one file per language, template overrides from `theme/templates/`.
- Rules: BL-WEB-1 to BL-WEB-6, BL-SEC-10, BL-TXT-3, BL-TST-7. `FRONTEND.md`.
- Done when: every page type of the demo site has an `insta` snapshot, passes the HTML validator, and reads completely with scripts off.

### 1.12 CSS and images

- Status: `todo`
- One public stylesheet under 20 KB compressed, critical CSS inline, tokens from `theme/tokens.toml`. Images resized and converted to WebP and AVIF at publish, never at request, with `width`, `height`, `srcset` and `sizes`.
- Rules: BL-WEB-5, BL-WEB-7, BL-PERF-5, BL-RUN-7.
- Done when: the CSS budget is measured in `scripts/perf.sh` and holds, and every rendered image has its four attributes.

### 1.13 SEO outputs

- Status: `doing`. Part 1 (this landing): the `khalid-seo` crate, its own input types (ADR 0009), clean page paths and page addresses for the mount path and language. Next: title and description templates; robots directives; head tags; Open Graph, Twitter cards, verification and hreflang; sitemaps; robots.txt, `llms.txt` and redirects; feeds; the link policy.
- `crates/khalid-seo`: title and description tags, canonical, hreflang pairs from the `translation` key, Open Graph, sitemaps per language with `lastmod`, RSS, Atom and JSON feeds per collection and channel, and `robots.txt`.
- Rules: BL-TYP-5, BL-PERF-2. Plan section 5.2.
- Done when: snapshots cover each output for the demo site, every hreflang pair is complete, and a sitemap lists exactly the published, indexable pages.

### 1.14 Schema markup

- Status: `todo`
- JSON-LD graphs for Article, BlogPosting, Person, Organization, BreadcrumbList, WebSite, DefinedTerm and DefinedTermSet, TechArticle, and the schema each block from 1.7 writes (FAQ, how-to, review, recipe, video, business card).
- Rules: plan section 5.9.
- Done when: each graph has a snapshot and passes a JSON-LD structure test against the types Google's Rich Results Test accepts.

## Serving

### 1.15 The server and `khalid serve`

- Status: `todo`
- `crates/khalid-server`: axum, clean URLs, pages rendered at publish and cached until a file changes, ETags, Brotli, security headers, a CSP with a per-request nonce, file watching, graceful shutdown. `khalid serve --site <folder>` runs it.
- Rules: BL-SEC-2, BL-RUN-6, BL-RUN-7, BL-RUN-10, BL-PERF-2, BL-PERF-3.
- Done when: an end-to-end test starts `khalid serve` on the demo site and fetches every page type, each with the CSP header, and an edited post appears without a restart.

### 1.16 `khalid init`, `new` and `build`

- Status: `todo`
- `init` asks the four questions from plan section 2 (address, mount, languages, mode) and writes a site folder; with `--yes` or no terminal it takes flags instead. `new "Title"` writes a post with correct front matter and a transliterated slug. `build` renders the whole site.
- Rules: BL-CLI-1 to BL-CLI-4, BL-RUN-9.
- Done when: CLI tests run each command in human and `--json` output with the right exit codes, and `init` then `new` then `build` produces a site that `serve` serves.

## The API

### 1.17 The Content API

- Status: `todo`
- `/api/content/v1/` for posts, pages, authors, tags, categories, collections, glossary, case studies, changelog, settings and navigation, with the shared query grammar (`filter`, `include`, `fields`, `formats`, `limit`, `page`, `order`) parsed by one module, content keys, ETags, `Accept-Version`, and the OpenAPI document at `/api/openapi.json` generated with `utoipa`.
- Rules: BL-API-4, BL-API-5, BL-ERR-5. `docs/standards/api.md`.
- Done when: a contract test fails when routes and the OpenAPI document disagree, the filter grammar has property tests, and the demo posts come back through the API.

### 1.18 Keys and the Admin API

- Status: `todo`
- Integration keys and staff tokens with scopes, 256 random bits, stored hashed, compared in constant time. `khalid keys create` and `revoke`, showing a key once through hidden output. `/api/admin/v1/` creates and edits posts from markdown or HTML, with idempotency keys and a 409 on a stale `updated_at`. Writes are atomic.
- Rules: BL-SEC-3, BL-API-7, BL-RUN-5, BL-RUN-9, BL-CLI-4.
- Done when: HTTP tests cover success, validation failure, auth failure and a stale edit for each endpoint, and no key appears in any log or test output.

### 1.19 Webhooks

- Status: `doing`. Part 1 (#41): `Outbound::post`, which deliveries go through. Part 2 (this landing): `crates/khalid-webhooks` with the event catalogue (ADR 0010). Part 3: signing. Part 4: the verifier for incoming webhooks. Part 5: delivery, retries and the delivery log.
- The events in plan section 5.12, signed with HMAC-SHA256 over timestamp and body, sent through `Outbound`, retried with backoff over 24 hours, with a delivery log in the database.
- Rules: BL-SEC-7, BL-RUN-8.
- Done when: `wiremock` tests show each event fires once, is signed correctly, and retries after a failure.

### 1.20 The TypeScript SDK

- Status: `todo`
- A TypeScript client generated from the OpenAPI document, in `sdk/typescript/`, strict mode, Biome, with a test against a running `khalid serve`.
- Rules: `docs/standards/typescript.md`, BL-API-4.
- Done when: regenerating the SDK from the current document shows no diff in CI, and the SDK fetches the demo posts.

## Proof

### 1.21 Performance at scale

- Status: `todo`
- A generated 1,000-post and a 10,000-post fixture, `divan` benchmarks for render and lint, and `scripts/perf.sh` measuring cold start to first byte, idle memory, render p50 and p99, full and incremental rebuild, against `perf/baseline.json`.
- Rules: BL-PERF-1, BL-PERF-4, BL-FS-4.
- Done when: every Phase 1 budget in `docs/standards/performance.md` is measured in CI and holds.

### 1.22 The demo, end to end

- Status: `todo`
- A CI job that serves the demo site, runs Lighthouse, the HTML validator and axe on every page type, and repeats with scripts off at six widths. Headless browsers run in CI only, never on the 8 GB laptop.
- Rules: BL-WEB-1 to BL-WEB-5, BL-PERF-1.
- Done when: Lighthouse scores 100 in all four categories on the default post page, and the "Shows the founder" line above works from a fresh clone.

## Questions for the founder

None open.

## Order

1.1 to 1.4 first, since everything else depends on them. Then 1.5 to 1.10 for content, 1.11 to 1.14 for output, 1.15 and 1.16 for serving, 1.17 to 1.20 for the API, and 1.21 and 1.22 to prove it. 1.4 can move later if a content task is blocked, because only webhooks need it before 1.21.
