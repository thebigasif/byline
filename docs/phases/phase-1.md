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

- Status: `done` (#50, #68, #122, #127, #136 and this landing). KH-CON-5 for the standard blocks, callout and figure; the rest of the blocks are 1.7.
- `comrak` with footnotes, tables and task lists, plus the directive syntax every editor block uses. The round-trip harness: markdown to blocks to markdown, byte for byte. First blocks: heading, paragraph, image with required alt and caption, quote, code with syntax colours, divider, table, footnote, callout.
- Rules: BL-FS-5, BL-TST-5, BL-SEC-1.
- Done when: every listed block has a directive, a round-trip test and an HTML snapshot, and raw HTML in markdown is escaped.

### 1.7 The remaining blocks

- Status: `doing`. 1.7a1 (#175): the one-line leaf form (ADR 0018), toggle, table of contents and button. Checks (this landing): `khalid --site` reads every block in every body and refuses a broken one with the file and line, with a `fixtures/invalid/block-*` folder per refusal (#178). 1.7a2: bookmark, drawn as a card when publish fills its title. 1.7a3 (this landing): embed, a player in a frame for the six providers in `EMBED_PROVIDERS`, or a plain link for any other address. 1.7b media, 1.7c faq, how-to and review, 1.7d recipe, product, business card and header, 1.7e html, form, newsletter and gated content.
- The rest of plan section 4.6's block list, in parts of about five blocks each: gallery, video, audio, file download, bookmark card, product card, toggle, button, header, embed, HTML with visibility rules, FAQ, how-to, review, recipe, business card, form, newsletter signup, gated content, table of contents. Blocks that need a server feature (form, newsletter, gated content) render their plain HTML form and lock card for now.
- Rules: BL-FS-5, BL-TST-5, BL-SEC-1.
- Done when: every block in section 4.6 has a directive, a round-trip test and a snapshot.

### 1.8 Wikilinks and maths

- Status: `todo`
- `[[slug]]` resolves to posts, pages and glossary terms at build, in the same language, and reports broken ones. Maths renders on the server as MathML with `latex2mathml`.
- Rules: BL-PERF-2.
- Done when: the demo site's two glossary wikilinks resolve in both languages, a broken one is reported with file and line, and a maths fixture renders to MathML with no JavaScript.

### 1.9 Routes, collections and channels

- Status: `done` when part 4 lands. Part 1 (#80): `crates/khalid-routes` reads collections and their permalink patterns (`{slug}`, `{year}`, `{month}`, `{day}`, `{collection}`) from `routes.toml`, with the line and field of each mistake. Part 2 (#81): taxonomy prefixes, channels with filters, and `Routes::load`, which gives the defaults when there is no file. Part 3 (#83): `khalid --site` checks `routes.toml` after `site.toml` and before the content, with five `fixtures/invalid/routes-*` folders, one per mistake. Part 4 (this landing): `Routes::address` builds a post's address from its pattern, the mount path and the language prefix, and `Routes::redirect` turns an alias into a redirect; `crates/khalid-routes/tests/addresses.rs` covers the done line. Still open, outside this track: the `aliases` and `featured` front matter fields in `khalid-content`, and writing alias redirects into the index during a rebuild once they exist. The glossary, case studies and changelog schemas are in task 1.5.
- `routes.toml` per plan section 4.11: collections with URL patterns, channels, taxonomy prefixes, aliases, mount path and language prefix. Defaults need no file. The glossary, case studies and changelog collections from section 5.13 with their typed front matter.
- Rules: BL-FS-3. Plan sections 4.11 and 5.13.
- Done when: URL tests cover the default `/blog/{slug}/`, a `/news/{year}/{slug}/` collection, `/de/` prefixes, a mount path, and an alias that becomes a redirect.

### 1.10 The database

- Status: `done` when part 7 lands. Part 1 (#39): `crates/khalid-db`, opening the file with WAL, foreign keys, a busy timeout, one writer and a read pool, and forward-only migrations. Part 2 (#40): the derived tables of the content index, ADR 0010, and a test that the crate README lists every table as derived or visitor. Part 3 (#42): the visitor tables, none of which points at a derived table. Part 4 (#51): `Database::site` and a rebuild that empties a site's derived tables in one transaction and keeps its visitor rows, with checked queries compiled from `.sqlx/`. Part 5 (#55): posts written into the index, listed newest first, and an `EXPLAIN QUERY PLAN` test that the list reads `posts_by_date` and never sorts. Part 6 (#59): `khalid --site <folder> db migrate`. Part 7 (this landing): `khalid --site <folder> db rebuild` indexes every post from the content files, and `crates/khalid/tests/rebuild.rs` deletes `data/site.db`, rebuilds, and gets the same index with `content/` unchanged.
- `crates/khalid-db`: SQLite in WAL mode with foreign keys, a busy timeout, one writer and a read pool; forward-only migrations; the index rebuilt from content files on demand. Nothing derived is written under `content/`.
- Rules: BL-DB-1 to BL-DB-3, BL-FS-1, BL-FS-2, BL-RUN-3.
- Done when: deleting `data/site.db` and starting again rebuilds the same index, checked by a test.

## Output

### 1.11 Templates

- Status: `doing`. Part 1 (#43): `crates/khalid-render` and its input records. Part 2 (#45): strings per language, text direction, HTML escaping. Part 3 (#46): the renderer, the base layout with the head and share card, and the 404 page. Part 4 (#47): the snapshot and HTML validity checker every page type passes. Part 5 (#48): templates in `theme/templates/` replace the defaults, with the sandbox tests. Part 6 (#52): the post page, with dates in the page's language. Part 7 (#57): responsive images as `picture` with AVIF and WebP, and the post's lead image. Part 8 (#58): the home page and collection lists with pagination. Part 9 (this landing): tag and category lists, the author page and plain pages. Next: the strings, the renderer and its sandbox, theme replacements, then each page type.
- `crates/khalid-render`: sandboxed MiniJinja with autoescape, default templates for home, post, page, author, tag, category, collection index, glossary A to Z and 404, semantic HTML with one `h1`, `lang` and `dir`, strings in one file per language, template overrides from `theme/templates/`.
- Rules: BL-WEB-1 to BL-WEB-6, BL-SEC-10, BL-TXT-3, BL-TST-7. `FRONTEND.md`.
- Done when: every page type of the demo site has an `insta` snapshot, passes the HTML validator, and reads completely with scripts off.

### 1.12 CSS and images

- Status: `doing`. Media part 1 (this landing): `crates/khalid-media` with the content key, the widths an image is offered in, and the cache under `data/media/` with atomic writes (#93). Media part 2 (this landing): decoding by content, Lanczos3 resizing, and WebP, JPEG and PNG encoding. AVIF waits on the dependency question below (#98). Media part 3 (this landing): `derive` makes every variant in a rayon pool and skips files the cache has; `derive_in_background` runs it in `spawn_blocking` (#100). Media part 4 (this landing): the divan benchmark for the resize path. Left for 1.12: AVIF (the question below), and the CSS measure in `scripts/perf.sh`, which this track may not edit. The stylesheet parts are stacked on the templates track (#75, #78).
- One public stylesheet under 20 KB compressed, critical CSS inline, tokens from `theme/tokens.toml`. Images resized and converted to WebP and AVIF at publish, never at request, with `width`, `height`, `srcset` and `sizes`.
- Rules: BL-WEB-5, BL-WEB-7, BL-PERF-5, BL-RUN-7.
- Done when: the CSS budget is measured in `scripts/perf.sh` and holds, and every rendered image has its four attributes.

### 1.13 SEO outputs

- Status: `doing`. Parts 1 to 4: the `khalid-seo` crate and its own input types (ADR 0009), page addresses, title and description templates, robots directives, head tags. Part 5: Open Graph and Twitter cards with a default share image and a per-page override. Part 6: verification tags for Google, Bing, Yandex, Baidu and Pinterest, and verification files at the root. Part 7: hreflang with `x-default` from translation groups, the same complete set on every version. Part 8a: test checks for the rules of the sitemap schema. Part 8b: the XML sitemap of exactly the published, indexable pages, with language alternates. Part 9a: images and videos in the sitemap. Part 9b: the news sitemap. Part 10a: the sitemap index at `sitemap.xml`, checked against the index schema's rules. Part 10b: one sitemap file per section and language, split at 50,000 addresses or 50 MiB. Part 10c: the HTML sitemap model per language. Part 11a: `robots.txt` with per-bot allow and block, checked with Google's parser. Part 11b: `llms.txt`. Part 12a: the feed model per language and category, full or excerpt, with text before and after items and a source line. Part 12b: RSS 2.0, checked with a strict reader and validator. Part 12c: Atom 1.0. Part 12d: JSON Feed 1.1. Part 13a: the redirect table with 301, 302, 307, 410 and 451, chains collapsed into one hop, loops refused, and a debug view. Part 13b: regex redirect rules. Part 14 (this landing): the link policy, the last part. Every 1.13 output now has its snapshot and its validator; the item left is wiring the inputs from content and `Config`, which is task 1.15. The share image generated when none is set is image work for 1.12 or Phase 2.
- `crates/khalid-seo`: title and description tags, canonical, hreflang pairs from the `translation` key, Open Graph, sitemaps per language with `lastmod`, RSS, Atom and JSON feeds per collection and channel, and `robots.txt`.
- Rules: BL-TYP-5, BL-PERF-2. Plan section 5.2.
- Done when: snapshots cover each output for the demo site, every hreflang pair is complete, and a sitemap lists exactly the published, indexable pages.

### 1.14 Schema markup

- Status: `doing`. Part 1: the JSON-LD graph model in `khalid-seo`: typed values, nodes with `@id` references, one `@graph` per page, and a script tag no text can close. Part 2a: the publisher as Organization or Person, and WebSite with a SearchAction. Part 2b: BreadcrumbList on every page. Part 3a: the whole graph of a page, with its page node typed by kind and Speakable. Part 3b: Article, BlogPosting, NewsArticle and TechArticle by content type. Part 3c: the author as Person with job title, bio, `sameAs` and `knowsAbout`, and `about` and `mentions`. Part 4a: DefinedTerm and DefinedTermSet for the glossary. Part 4b: LocalBusiness with one or several locations. Part 5a: the FAQ block as FAQPage, and ADR 0015 on which blocks are typed in Phase 1. Part 5b: the how-to block as HowTo, and money in minor units. Part 5c: the video and review blocks as VideoObject and Review with AggregateRating. Part 5d (this landing): the recipe block as Recipe. Next: schema templates and custom JSON-LD; the schema.org vocabulary and Google's required and recommended properties.
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

- Status: `doing`. Part 1 (#84): `crates/khalid-api` with the `filter` parser for one condition. Part 2 (#86): every operator, `+`, `,` and brackets, with limits on length, depth and size. Part 3 (#92): printing filters so they parse back, and matching them against a resource. Part 4 (#95): `filter`, `formats`, `limit` and `page` read into one `ListQuery`. Part 5 (#97): `include`, `fields` and `order`. Part 6 (#101): RFC 9457 problem details with `fix` and `rule`. Part 7 (#103): `meta.pagination`. Part 8 (#107): the `Post` resource in API shape, with its filter fields. Part 9 (#113): the `ContentStore` trait and an in-memory store. Part 10 (#118): `GET /posts/` and `/api/openapi.json`, generated with utoipa-axum. Part 11 (#119): reading one post by id or slug. Part 12 (#126): the key format, the `KeyStore` trait and content key checks (ADR 0011). Part 13 (#130): every Content API route needs a content key, in the `Authorization` header or `?key=`. Part 14 (#133): ETags with 304s, cache headers, `Accept-Version` and `Content-Version`. Part 15 (#139): the content store keeps entries per collection: posts, pages, glossary, case studies and changelog. Part 16 (#140): the three routes for each of them. Part 17 (this landing): the contract test that fails when routes and the `OpenAPI` document disagree. Next: authors, tags and categories, then settings, navigation and collections.
- `/api/content/v1/` for posts, pages, authors, tags, categories, collections, glossary, case studies, changelog, settings and navigation, with the shared query grammar (`filter`, `include`, `fields`, `formats`, `limit`, `page`, `order`) parsed by one module, content keys, ETags, `Accept-Version`, and the OpenAPI document at `/api/openapi.json` generated with `utoipa`.
- Rules: BL-API-4, BL-API-5, BL-ERR-5. `docs/standards/api.md`.
- Done when: a contract test fails when routes and the OpenAPI document disagree, the filter grammar has property tests, and the demo posts come back through the API.

### 1.18 Keys and the Admin API

- Status: `doing`. Part 1 (#147): integration keys and staff tokens as key kinds, with scopes and roles. Part 2 (#148): stored as Argon2id, checked off the async runtime, and `authenticate` returns what the key may do (ADR 0011). Part 3 (#153): one router and one `OpenAPI` document that every API joins. Part 4 (#155): `AdminPost`, a post in any status, and the `PostStore` trait. Part 5 (#156): the Admin API under `/api/admin/v1/`, its key layer, scope checks with a 403, and reading one post. Part 6 (#160): editing a post, with a 409 naming who changed it and when on a stale `updated_at`. Part 7 (this landing): creating a draft from markdown, once per `Idempotency-Key`. Next: `source=html`.
- Integration keys and staff tokens with scopes, 256 random bits, stored hashed, compared in constant time. `khalid keys create` and `revoke`, showing a key once through hidden output. `/api/admin/v1/` creates and edits posts from markdown or HTML, with idempotency keys and a 409 on a stale `updated_at`. Writes are atomic.
- Rules: BL-SEC-3, BL-API-7, BL-RUN-5, BL-RUN-9, BL-CLI-4.
- Done when: HTTP tests cover success, validation failure, auth failure and a stale edit for each endpoint, and no key appears in any log or test output.

### 1.19 Webhooks

- Status: `done` after this landing. Part 1 (#41): `Outbound::post`, which deliveries go through. Part 2 (#49): `crates/khalid-webhooks` with the event catalogue (ADR 0010). Part 3 (#53): signing. Part 4 (#56): the verifier for incoming webhooks. Part 5 (#61): the delivery log model and the retry schedule over 24 hours. Part 6 (#70): the `DeliveryStore` trait with an in-memory store. Part 7 (#74): the `Dispatcher`, which signs, sends through `Outbound` and logs. Part 8 (this landing): retrying what is due. Task 1.15 implements `DeliveryStore` on SQLite and calls `retry_due` on a timer.
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

## Phase 2 work pulled forward

The linter depends on nothing in progress, so it is built now, in parallel, from plan section 6 and KH-LNT-1, KH-LNT-2, KH-LNT-3 and KH-LNT-6. `crates/khalid-lint` depends on `khalid-core` only and reads its own input model (ADR 0011). Each part lands as its own pull request.

### 2.1 The rule engine

- Status: `done` after this landing. Part 1 (#106): the crate and its input model. Part 2 (#111): the message catalogue and the messages test. Part 3 (#112): severities, the rule trait and `lint.toml`. Part 4 (this landing): the engine, its findings and its report.
- The `Rule` trait, `Severity` (block, warn, off), a `Finding` with its rule, severity, location and a three-sentence `Message` from `catalogue.toml`, `lint.toml` loading with errors that name the line and column, and the input model: `LintPost`, `LintSite` and the `SiteView` trait the content track implements in 1.15.
- Rules: BL-ERR-1, BL-TXT-2, BL-TST-4, BL-TYP-5.
- Done when: the messages test from `docs/standards/testing.md` walks the catalogue (three sentences, each under 25 words, no em dash, no jargon, the fix names an action), and a mistake in `lint.toml` names its line and column.

### 2.2 The five blocking rules

- Status: `done` after this landing. Fixtures are `post.toml` files in the lint input model, because the crate reads no markdown; `tests/fixtures.rs` fails if a catalogue rule has no fixture.
- No date, no author, no internal link, no title, more than one main headline. Each has a fixture under `crates/khalid-lint/fixtures/<rule>/` with a post and `expected.txt`. The three related posts the plan suggests with "no internal link" came in 2.5, part 5.
- No date, no author, no internal link, no title, more than one main headline. Each has a fixture under `crates/khalid-lint/fixtures/<rule>/` with a post and `expected.txt`.
- Rules: BL-TST-4. KH-LNT-1.
- Done when: each fixture produces exactly its expected findings, and a clean post produces none.

### 2.3 The link text rules

- Status: `done` after this landing. Link words compare without case, spacing or a final full stop, and targets without a `#section` or trailing slash. "Too often" means more than three links site-wide, tunable as `same-link-text-max` in `lint.toml` since 2.4. Vague words are listed for English and German.
- Link text reused for a different page, the same link text to the same page too often, vague link text, the same target twice in one post, a link with no text.
- Rules: BL-TST-4. KH-LNT-2.
- Done when: one fixture per message, each with its expected findings.

### 2.4 The structure rules

- Status: `done` after this landing. Part 1 (#129): the `[thresholds]` table in `lint.toml`, starting with the link limit from 2.3. Part 2 (#131, with `thin-post-words`): heading order, missing image descriptions, thin post, no outbound link. Part 3 (#134): a missing description, title and description length and duplicates in the same language. Part 4 (this landing): keyword stuffing for phrases of two or three words, slug length, filler words and years, broken wikilinks. The warning for a Unicode slug on an ASCII site (CTO answers, section 2) needs the site's slug mode, so it comes with the wiring in 1.15.
- Heading order, title and description length and duplicates, missing alt text, thin post, keyword stuffing, slug hygiene, no outbound link, broken wikilink.
- Rules: BL-TST-4. KH-LNT-2.
- Done when: one fixture per message, each with its expected findings.

### 2.5 The site rules

- Status: `done` after this landing. Part 1 (#143): the `SiteRule` trait, which checks the whole site in one pass; `Linter::site_health` for the dashboard; the link graph; orphan post. A post may carry its `path`, so links find it by path. The publish check, `lint_post`, never runs site rules, because a new post has no inbound links yet. Part 2 (#145): one inbound link, and too deep, counted in clicks from the home page's links (`max-clicks`, default 3). Part 3 (#149): stale post, measured from today's date read once through `Clock` when the linter starts (`stale-months`, default 12), and missing translation across the languages the site declares. Fixtures may carry a `lint.toml`. Part 4 (#151): near-duplicates by five-word shingles (`near-duplicate-percent`, default 50), glossary term not linked, cluster without its pillar. These are post rules, because a writer can fix each one before publishing. Part 5 (this landing): the three suggested posts for "no internal link", ranked by the topic words they share with the post, carried on the finding as `suggestions`.
- Orphan post, one inbound link, missing translation, near-duplicate posts by shingles, glossary term not linked, stale post, cluster without its pillar, too deep.
- Rules: BL-TST-4, BL-RUN-1, BL-TST-2. KH-LNT-2.
- Done when: one fixture per message, each with its expected findings, and the stale rule reads time through `Clock`.

### 2.6 `khalid lint` and the publish gate

- Status: `doing`. Part 1 (#158, with the grammar fix in #164): `Linter::gate`, the publish gate the server calls, returning a `Verdict` of publish or refuse with every finding; findings and reports serialise to the JSON the CLI, the API and the admin share. Part 2 (#166): `khalid --site <folder> lint` and `--json` in the binary, over every post, with exit code 1 on a block. The binary maps parsed posts and their rendered output to the lint model. On the demo site it reports 0 blocks. Part 3 (this landing): one post by its path (post rules only, as at publish), the site's `lint.toml` with mistakes named by line, pages, glossary terms and languages in the model, and the subject and suggestions of each finding in the sentences. Part 4: `--fix` for the safe repairs.
- `khalid lint [path] --json --fix` in the binary, exit code 1 on a block, and the publish gate function the server calls.
- Rules: BL-CLI-1, BL-CLI-2, BL-CLI-3. KH-LNT-3.
- Done when: CLI tests cover human output, `--json`, `--fix` and each exit code on the fixtures.

## Phase 3 work pulled forward

Theme sync and the generated share image depend on nothing in progress, so they start now. Plan sections 4.8 and 4.9, PRD KH-WEB-8, KH-INS-3, KH-SEO-2 and KH-OPS-2. Each part lands as its own pull request, branched from `main`.

### 3.1 Fetch the site

- Status: `done` after this landing. Part 1 (#170): `crates/khalid-theme`, and the stylesheets a page uses, in page order, same site only (ADR 0015). Part 2 (this landing): fetching them through `Outbound` within the caps.
- Read the home page and its same-site stylesheets through `Outbound`, with a cap on the page, on the CSS in all and on the number of stylesheets. Keep the raw CSS. A stylesheet that fails is listed with the reason; only the home page can stop a sync.
- Rules: BL-RUN-8, BL-TST-3.
- Done when: `wiremock` tests cover each cap, a link to another site that is never fetched, a failing stylesheet, a private address and an unreachable name.

### 3.2 Extract the tokens

- Status: `doing`. Part 1 (this landing): CSS colour values and their WCAG contrast. Next: the CSS reader and custom properties; the colours by frequency and contrast; fonts, logo and icons; all of it from one site.
- CSS custom properties, `@font-face` rules and Google Fonts links; the background, text, link and accent colours by frequency and contrast; the logo from `og:image` and the header image; the icon links.
- Done when: fixtures of saved real-world CSS each give the expected tokens.

### 3.3 Write `theme/tokens.toml` and a preview

- Status: `todo`, after 3.2 and the renderer's tokens loader (#72).
- The file in the shape the renderer loads, written atomically, and a preview page rendered through `khalid-render`.
- Rules: BL-RUN-9, BL-SEC-4.
- Done when: a synced file loads with the renderer's rules and the preview passes the HTML checks.

### 3.4 The contrast pass

- Status: `todo`, after 3.2.
- A text and background pair that fails WCAG 2.2 AA gets the nearest passing shade, and the user is told in plain words.
- Rules: BL-WEB-4, BL-TXT-2.
- Done when: every failing pair in the fixtures passes after the fix, with the smallest change of shade.

### 3.5 Re-sync with a diff

- Status: `todo`, after 3.3.
- `theme sync` again shows what changed, and keeps every field named in the file's `locked` list.
- Done when: a test edits a field by hand, locks it, re-syncs against a changed site and finds the hand edit kept and the rest updated.

### 3.6 Proxy rules

- Status: `doing`, in its own pull request. `crates/khalid-mount`.
- The exact snippet for nginx, caddy, apache, traefik, cloudflare-worker, vercel, nextjs, netlify, webflow and shopify, for a given mount path. KH-INS-3.
- Done when: each stack has a fixture, and the snippet for every stack matches it byte for byte.

### 3.7 The generated share image

- Status: `doing`, in its own pull request. `crates/khalid-share-image`.
- A 1200 by 630 PNG from the title, the site name and the theme colours, drawn in pure Rust, cached by content hash. KH-SEO-2.
- Done when: a snapshot test pins the image, and the same input reads from the cache.

### 3.8 `khalid theme sync` and `khalid theme check`

- Status: `todo`, after 3.1 to 3.5.
- Both commands in the binary, with a what, why and fix message for each failure: the site is unreachable, there is no stylesheet, the address is private, a token is missing. KH-OPS-2.
- Rules: BL-CLI-1 to BL-CLI-4, BL-ERR-4.
- Done when: CLI tests run both commands against a `wiremock` site, in human and `--json` output, with the right exit codes.

## Questions for the founder

- **AI crawlers in a new site's robots.txt (task 1.13).** Context: AI companies send crawlers that copy blog posts to train their models (GPTBot, CCBot and others), and separate crawlers that fetch posts to quote and link them in AI answers (OAI-SearchBot, Claude-SearchBot, PerplexityBot). Khalid can block or allow each one by name. Today a new site allows all of them. Consequence: blocking the training crawlers keeps posts out of future models and costs no search traffic; blocking the answer crawlers also removes the blog from AI answers. Question: should a new site block the training crawlers by default, and keep the answer crawlers allowed?
- **AVIF images need three exceptions in `deny.toml` (task 1.12, for the CTO session).** Every pure-Rust AVIF encoder is built on `rav1e`, which pulls in `libfuzzer-sys` (licence `(MIT OR Apache-2.0) AND NCSA`, not on our list), `av1-grain`, which uses `anyhow` (banned outside the binary), and `paste` (RUSTSEC-2024-0436, unmaintained). Why it matters: AVIF images are about half the size of JPEG, which helps the LCP budget, but the policy refuses these crates today. The choice: allow NCSA, add `av1-grain` as an `anyhow` wrapper and accept the `paste` advisory with a review date; or ship WebP with JPEG and PNG fallbacks only, which is what `khalid-media` does now. WebP alone already meets the budgets on the demo site. Adding AVIF later is one small pull request, because `Format` is non-exhaustive and the render side already accepts AVIF sources.
- **Members-only HTML is hidden from everyone for now (task 1.7e1).** The HTML block takes `show="everyone"`, `"visitors"` or `"members"`. Readers cannot sign in until the server and accounts exist, so Khalid leaves members-only HTML out of every page and always shows visitors' HTML. Why it matters: nothing private can leak before sign-in exists. The choice: keep this, or refuse `show="members"` until accounts ship. Either is one line to change.
- **Where `khalid keys create` stores a key (task 1.18, for the CTO session).** Context: the Admin API checks integration keys and staff tokens against an Argon2id hash looked up by the key's 16-digit identity (ADR 0011). The `api_keys` table in `khalid-db` holds a 32-byte `key_hash` and no identity, so it cannot hold either key. Why it matters: `khalid keys create` and `revoke` need a real store before they can ship with an end-to-end test, and `khalid-db` and the 1.15 wiring belong to other tracks. `khalid-api` now makes keys, shows them once, and gives `KeyStore` the `create` and `revoke` methods a store must have. The choice: the database track adds a migration with a `key_id` text column and a text `key_hash` and implements `KeyStore` in 1.15, after which the API track adds the two commands; or the API track may add that migration itself.

- **File watching could use `notify`, whose licence is CC0-1.0 (task 1.15, for the CTO session).** `docs/standards/rust.md` names `notify` for file watching. Every release of it is CC0-1.0, which puts the code in the public domain, but `deny.toml` does not list CC0-1.0, so the server watches by looking at the folder every half second instead (ADR "the server watches files by polling"). Why it matters: polling works everywhere, including Docker volumes and network drives, and costs a few milliseconds every half second on a 1,000-post site. Native events would show an edit a little sooner and cost nothing while idle. The choice: add CC0-1.0 to the allow list and switch `watch.rs` to `notify`, a small pull request, or keep polling.
- **A saved post takes about a third of a second to show, and the budget is 50 ms (task 1.21, for the founder and the CTO session).** CI now times it on the 1,000-post site: 340 ms from saving a post to its page changing. Why it matters: the budget in `docs/standards/performance.md` fails, so task 1.21 cannot be done. Two causes, and both need fixing. First, the server looks for changes every half second, so a save waits 250 ms on average before anything happens. Only file events fix that, and they need the `notify` answer above. Second, every save renders the whole site again, about 100 ms. Fixing that means rendering only the pages one post feeds, which reaches into the content track's crate. The choice: allow `notify` and plan partial re-rendering as its own task, or set a different budget for a saved post to show. Writers switching to a browser rarely notice a third of a second.
- **`khalid serve` got slower to start and holds more memory since #273 (task 1.21, for the CTO session).** On the 1,000-post site, CI measured these before #273: first page 108 ms (limit 110), idle memory 58 MB (limit 55), a saved post 340 ms (limit 50). After #273: first page 313 to 390 ms, idle memory 81 to 119 MB, a saved post 369 to 826 ms. On the 10,000-post site, an edit now takes about 17 s to show, up from 2 s. Why it matters: these three budgets cannot pass, so this stack stays red in CI whatever it changes in the server. The cause is `published` in `crates/khalid/src/site_content.rs`, which runs on every render. It reads the content index from the files again, rebuilds it in `data/site.db`, renders every post body a second time, and finds each post's file with a scan over all posts, so the work grows with the square of the number of posts. That code belongs to the API and database tracks, so this track has not changed it. The choice: let the API track refresh the index only for the files that changed, or after the first page is served, and replace the scan with a lookup by path; or let this track make that change.
- **Idle memory is 60 to 64 MB with the allocator tuned as far as it goes, and the limit is 55 MB (task 1.21, for the CTO session).** Measured as the CTO answers of 2026-10-27 define it, on the 1,000-post site. No mimalloc setting lowers it, and the system allocator holds as much, so the memory is data the server keeps, not memory the allocator fails to hand back (`docs/standards/performance.md`, "Allocator settings"). The rendered pages are 6.9 MB and the server's fixed cost is about 35 MB. Why it matters: the idle budget still fails, so task 1.21 cannot be done. The choice: once the API track's store change (answer 3) lands, this track measures what each part holds and trims the largest; or the budget for 1,000 posts is raised. This track suggests waiting for answer 3 before deciding.
- **The last part of 1.15, `khalid serve` and its end-to-end test, is blocked on the page wiring (for the CTO session).** Everything the server needs is in the stack ending at `task/p1-server-retry`: pages behind the `khalid_server::Render` trait, ETags, Brotli, the CSP nonce, the watch, the SQLite delivery log, the retry timer and the graceful stop. What turns the site's content into rendered pages is the build track's `crates/khalid/src/build_pages.rs`, on `task/p1-cli-build` and the branches above it, which have not reached `main`. Why it matters: the done line asks for every page type of the demo site through `khalid serve`, and this track may not stack on another track's branch or write that wiring itself. The plan: once `khalid build` lands, a pull request on this stack implements `Render` in the binary over those pages, adds `khalid serve --site <folder>` with the watch, the retry timer, Ctrl+C and the database closed last, and adds the end-to-end test that edits a demo post. Should the server track write the wiring instead, say so.
- **Should `khalid new` write a draft (task 2.6)?** `khalid new` dates the post it writes, and a dated post is published by `khalid build`. A new post has no author and no internal link yet, so the publish gate now refuses the next build until the writer adds both, or removes `date:` to keep it as a draft. Why it matters: the first build after `khalid new` fails, with a clear message, which may surprise a new user. The choice: keep it, so the gate teaches the rules from the first post; or have `khalid new` leave the date out, so the post is a draft until `khalid publish` dates it (plan section 3).

## Order

1.1 to 1.4 first, since everything else depends on them. Then 1.5 to 1.10 for content, 1.11 to 1.14 for output, 1.15 and 1.16 for serving, 1.17 to 1.20 for the API, and 1.21 and 1.22 to prove it. 1.4 can move later if a content task is blocked, because only webhooks need it before 1.21.
