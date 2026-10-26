# Khalid: product requirements

Version 1.0 of this document, 2026-10-26. Owner: the founder. Written by the CTO session from `docs/plan.md`, the founder's decisions and the research in `docs/research/`. The plan says how we build; this says what must be true when we ship, as numbered requirements that tasks, tests and the changelog point at. A requirement is done when its acceptance line holds on `main`.

## 1. The problem

Businesses want a blog that ranks, on the website they already have, without running WordPress and its plugin stack. Today they choose between a heavy CMS they do not want, a static generator with no editor and no SEO help, or a hosted platform that cannot sit under their own domain. Every one of them gets the SEO basics wrong in the same ways: no internal links, repeated anchor text, missing schema, slow pages. People who run many sites repeat that pain once per site.

## 2. Who it is for

| Customer | What they need | How they measure success |
|---|---|---|
| The SEO or growth lead at a company | A blog on `company.com/blog` that scores clean in Ahrefs and Semrush and publishes fast | Rankings and organic traffic without a developer in the loop |
| The agency running many client blogs | One install per client in minutes, the same hygiene enforced everywhere, agents doing the repetitive work | Hours saved per client per month |
| The developer asked to "add a blog" | One binary, one command, a public API, no new database to run | Done in an afternoon, nothing to maintain |
| The writer | An editor that gets out of the way and tells them plainly when a post is not ready | Posts published without a back-and-forth with SEO |
| SerpDad itself | The publish target at the end of its content journey, and a free tool that teaches SEO and brings customers | Signups from the free tool, articles published through it |

## 3. Goals and non-goals

**Goals for 1.0**

1. A business adds a blog to any existing website in under ten minutes, and it looks like that website.
2. Every page the blog serves would pass an Ahrefs or Semrush site audit with zero errors and zero warnings.
3. A post cannot be published without a date, an author and an internal link, and every refusal teaches why.
4. Everything the admin can do, the command line, the MCP server and the public API can do.
5. The whole system is one binary with SQLite, under 30 MB, idle under 50 MB, rendering a post in under 5 ms at the 99th percentile.

**Non-goals for 1.0**

A page builder. Building the whole website. E-commerce. Paid memberships. A plugin API beyond webhooks. AI writing inside the product. Keyword research, rank tracking or backlink data, which are SerpDad's job. A hosted version. A mobile app. Dark mode in the admin.

## 4. Principles

- **Files are the truth.** Every post is a text file. The database holds only what is derived or what visitors create.
- **Semantic HTML, whole with scripts off, responsive from 320 px, WCAG 2.2 AA, zero layout shift.** Not a setting, the only output.
- **Every message a person sees says what happened, why it matters and what to do.** No codes, no jargon, no em dashes.
- **Teach, then enforce.** The linter blocks three things and explains everything else.
- **Light theme only in the admin.** The public blog follows the host site's colours.
- **One door.** The API is the only way in; the CLI, the MCP server, the admin and SerpDad are its clients.
- **Permissive dependencies, AGPL product.** So SerpDad can reuse the crates and nobody can sell a closed fork.

## 5. Requirements

Each requirement has an ID, a statement and an acceptance line. The phase column says where the plan builds it.

### 5.1 Install and mount

| ID | Requirement | Acceptance | Phase |
|---|---|---|---|
| KH-INS-1 | One static binary per platform, installable with one command, Homebrew or a scratch Docker image | A fresh Linux machine serves the demo site within ten minutes following only the README | 7 |
| KH-INS-2 | `khalid init` asks four things: site address, where the blog lives, languages, server or static | A site folder with `site.toml`, `routes.toml`, `theme/` and `content/` exists afterwards | 1 |
| KH-INS-3 | When the blog sits under a folder of a site built with something else, `init` prints the proxy rule for that stack: Nginx, Caddy, Apache, Traefik, Cloudflare Worker, Vercel, Next.js, Netlify, Webflow, Shopify | `khalid doctor` fetches the public address and confirms the rule works | 3 |
| KH-INS-4 | Built-in HTTPS through Let's Encrypt when given a domain | `khalid serve --domain x` answers over TLS with a valid certificate and renews it | 5 |
| KH-INS-5 | Headless mode: the theme switched off, everything else running, for sites that render posts in their own front end | The Astro and Next.js starter kits render the demo blog from the Content API with schema and meta intact | 1, 7 |

### 5.2 Content model

| ID | Requirement | Acceptance | Phase |
|---|---|---|---|
| KH-CON-1 | Posts, pages, authors, glossary entries, case studies, changelog entries and snippets are markdown files with YAML front matter under `content/<lang>/` | The demo site parses with zero errors; each type has a typed schema | 1 |
| KH-CON-2 | A collection folder may hold `_index.md` with its own front matter and body | The collection's list page shows that body | 1 |
| KH-CON-3 | A post may be a folder with `index.md` and its images beside it | Images resolve, and a changed image changes the post's content hash | 1 |
| KH-CON-4 | Unknown fields, wrong types and broken YAML are refused with file, line and field; a missing title, date or author is left for the linter | One fixture per mistake under `fixtures/invalid/` with its exact message | 1 |
| KH-CON-5 | Every editor block has a markdown directive, and parse then print returns the same text | A round-trip test per block | 1 |
| KH-CON-6 | Wikilinks `[[slug]]` resolve at build; broken ones are linted | Test per case | 1, 2 |
| KH-CON-7 | Slugs are ASCII by default with per-language transliteration (German ä ö ü ß to ae oe ue ss), with a per-site Unicode mode; 200 bytes at most, encoded | Property tests for both modes | 1 |
| KH-CON-8 | A language folder per language, a `translation` key pairing posts across languages, hreflang with x-default on every page | `khalid lint` warns when a declared language lacks a translation | 1, 2 |
| KH-CON-9 | Routing in `routes.toml`: collections with permalink patterns, channels with filters, taxonomy prefixes, front matter aliases that redirect | Located errors; a lint warning when two collections could claim one post | 1 |
| KH-CON-10 | Nothing derived is ever written under `content/`; derived data lives in `data/` and rebuilds from the files | Deleting `data/site.db` and starting the server restores every page | 1 |
| KH-CON-11 | A change to one file re-renders only what depends on it; a full rebuild of 10,000 posts takes under 30 seconds | Benchmark in CI | 1 |

### 5.3 Output and SEO defaults

| ID | Requirement | Acceptance | Phase |
|---|---|---|---|
| KH-SEO-1 | Titles and descriptions from templates per page type, with length and duplicate checks and a Google snippet preview | Lint rules and an editor preview | 2, 4 |
| KH-SEO-2 | Canonical, Open Graph and Twitter cards on every page, a default share image, a per-post override, and a generated share image when none is set | Snapshot per page type | 2 |
| KH-SEO-3 | JSON-LD for every page from the content: Article variants, Person, Organization or LocalBusiness, WebSite with SearchAction, BreadcrumbList, FAQPage, HowTo, VideoObject, Review, Recipe, ItemList, Event, Course, Product, SoftwareApplication, Book, PodcastEpisode, ClaimReview, Dataset, Speakable, about and mentions | Every graph validates; every type passes Google's Rich Results Test | 1, 2 |
| KH-SEO-4 | Schema templates per collection, a builder for any schema.org type, a custom JSON-LD field, a preview, and "copy schema from a URL" | Admin tests | 4 |
| KH-SEO-5 | Sitemaps for posts, pages, images, video and news, per language with alternates, split at 50,000, listed in robots.txt, plus an HTML sitemap; any post excludable | Validator test | 1 |
| KH-SEO-6 | RSS, Atom and JSON Feed per language and category, full or excerpt, with optional text before and after items and a source line | Feed validator test | 1 |
| KH-SEO-7 | robots.txt, `llms.txt`, per-bot allow and block, site verification tags for Google, Bing, Yandex, Baidu and Pinterest, and any verification file served from the root | Snapshot | 1, 2 |
| KH-SEO-8 | Redirects: 301, 302, 307, 410, 451, regex rules, CSV import and export, automatic 301 on slug change, chains collapsed, a debug view | Tests per type; a 404 log with one-click redirect | 2 |
| KH-SEO-9 | Per-post robots directives and a canonical override; index settings per archive type | Snapshot per setting | 2 |
| KH-SEO-10 | A site-wide link policy: nofollow, sponsored or ugc for external links, new-tab, affiliate domains always sponsored | Snapshot | 2 |
| KH-SEO-11 | IndexNow on publish, sitemap pings | Webhook stub test | 2 |
| KH-SEO-12 | Head and body code per site and per post, allowed through the Content Security Policy, with a consent banner that holds scripts until agreed | CSP header test on every page type | 5 |

### 5.4 The linter and the audit

| ID | Requirement | Acceptance | Phase |
|---|---|---|---|
| KH-LNT-1 | Publish is blocked when a post has no date, no author or no internal link; also for no title, more than one H1, or invalid schema | A fixture per block with its message | 2 |
| KH-LNT-2 | Warnings for: an anchor text reused for a different page, the same anchor to the same page too often, vague anchors, a target linked twice in one post, links to redirecting or dead URLs, orphan posts, heading order, title and description length and duplicates, missing alt, thin posts, keyword stuffing, hard reading, slug hygiene, missing translations, cross-site links in a multi-site process, no outbound link, broken wikilinks, a glossary term not linked, a form with no destination, overlapping collections, focus topic missing from title, H1, URL, intro, description or every heading, a cluster post without its pillar, nofollow internal links, too many links, one inbound link, too deep, links without anchor text, stale posts, near-duplicates, insecure internal links | One fixture per message; every message has what, why and fix, under 25 words each | 2 |
| KH-LNT-3 | Severities tunable in `lint.toml`; `khalid lint --json`; exit code 1 on a block; `--fix` for safe repairs | CLI tests | 2 |
| KH-LNT-4 | A focus topic per post, one primary keyword and related terms, and an SEO score out of 100 that summarises the checks and never replaces them | Admin and lint tests | 2, 4 |
| KH-LNT-5 | `khalid audit` crawls the site as Ahrefs and Semrush do, mirrors their checks in their categories, gives a health score with a trend, fix instructions and CSV export, and writes the internal linking report with click depth and link rank | The demo site scores 100 in Ahrefs and Semrush Site Audit with zero errors and warnings | 2 |
| KH-LNT-6 | The lint engine is a crate SerpDad's backend can depend on | Published crate with no AGPL-incompatible dependency | 2 |

### 5.5 The public site

| ID | Requirement | Acceptance | Phase |
|---|---|---|---|
| KH-WEB-1 | Semantic HTML with one H1, labelled landmarks, lang and dir on every page | W3C validator and axe with zero errors in CI | 1 |
| KH-WEB-2 | Every public page reads and every public form works with scripts off | Scripts-off run in CI | 1, 5 |
| KH-WEB-3 | Responsive from 320 px to 1920 px and at 200% zoom with no horizontal scroll; touch targets 44 px | Six widths in CI | 1 |
| KH-WEB-4 | WCAG 2.2 AA; contrast checked against the synced tokens | axe and contrast tests | 1, 3 |
| KH-WEB-5 | Zero layout shift; images with width, height, srcset, sizes and alt; fonts with tuned fallbacks | Lighthouse CLS 0 | 1 |
| KH-WEB-6 | One CSS file under 20 KB compressed, no framework, no third-party script by default, self-hosted subset fonts | Size gate | 1 |
| KH-WEB-7 | Lighthouse 100 in all four categories on the default post page; LCP under 1 second on a five-dollar server | Lighthouse CI | 1 |
| KH-WEB-8 | Theme sync pulls colours, fonts, logo and icon from the main site into `theme/tokens.toml`, fixes failing contrast pairs, re-syncs with a diff and keeps hand edits | Eight of ten real sites look native without edits | 3 |
| KH-WEB-9 | Navigation menus, design settings, announcement bar, linked mentions, hover previews that degrade to plain links, maths as MathML | Snapshots | 1, 4 |

### 5.6 Writing and collaboration

| ID | Requirement | Acceptance | Phase |
|---|---|---|---|
| KH-EDT-1 | A markdown-first editor storing markdown, with every block from the plan, a slash menu, markdown shortcuts, emoji autocomplete, link search, @-mentions, wikilinks, Unsplash and GIF search, basic image editing, snippets | Round-trip test per block; mockups approved before build | 4 |
| KH-EDT-2 | Autosave, an unsaved-changes guard, version history with diff and restore, presence and locking, a live preview in the real theme | Admin tests | 4 |
| KH-EDT-3 | Post settings: slug, date, co-authors, excerpt, feature image with caption and alt, featured flag, review state, access, template, canonical, robots, per-network share cards, aliases, scheduled unpublish, password, head code, an Updates list | Admin tests | 4 |
| KH-EDT-4 | Review states draft, in review, ready, published with role rules; comments on drafts; a calendar; git mode with branch per draft and pull on push | Admin and API tests | 4, 5 |
| KH-EDT-5 | A media library with folders, duplicate detection, focal points, S3-compatible storage | Admin tests | 4 |
| KH-EDT-6 | A logged-in toolbar on the public site; an audit log; trash with restore; bulk actions | Admin tests | 4 |
| KH-EDT-7 | Roles owner, admin, editor, writer, contributor; passkeys, email and password, magic link; a super-admin panel for many sites in one process | Auth tests | 4 |

### 5.7 Growth

| ID | Requirement | Acceptance | Phase |
|---|---|---|---|
| KH-GRO-1 | A form builder block with spam protection, a submissions table, email and webhook delivery | Submission reaches both in under 5 seconds | 4, 5 |
| KH-GRO-2 | Subscribers with segments and labels, several newsletters, open and click analytics, a welcome sequence, double opt-in, sends through SMTP, Resend or Postmark | End-to-end test with a stub provider | 5 |
| KH-GRO-3 | Email-to-unlock gated content; an embeddable signup form for the main website; CTA blocks with UTM tags; an announcement bar; RSS to email | Tests | 4, 5 |
| KH-GRO-4 | Privacy-friendly analytics with no cookies counted on the server, AI search referrers grouped, a crawl log of bots per page | Works with scripts off | 5 |
| KH-GRO-5 | An opt-in Search Console connection: per-post clicks, impressions, position, index status, winners and losers, decay alerts, 404s with traffic, a weekly email, and "Go deeper in SerpDad" links | Tests with a stub | 5 |
| KH-GRO-6 | Alerts by email or webhook for broken links, 404 spikes, certificate problems, failed scheduled posts, stale posts; CSV and JSON export of every table | Tests | 5 |

### 5.8 API and integrations

| ID | Requirement | Acceptance | Phase |
|---|---|---|---|
| KH-API-1 | Content API: read-only, Ghost's query shape (filter, include, fields, formats, limit, page, order), content key safe in browsers, cacheable with ETags | Contract tests | 1 |
| KH-API-2 | Admin API: read and write for everything, `source=html` on post creation, keys per integration with scopes, staff tokens, `updated_at` collision detection returning 409 | Contract tests | 1 |
| KH-API-3 | Webhooks for named events, HMAC-signed, retried with backoff, a delivery log | Each event fires once and retries on failure | 1 |
| KH-API-4 | An OpenAPI document served by the binary, a TypeScript SDK generated from it, Accept-Version headers, RFC 9457 errors with fix and rule fields | A contract test fails when routes and document disagree | 1 |
| KH-API-5 | An MCP server exposing every operation; `khalid agents` prints the embedded agent guide | MCP conformance test | 1, 7 |
| KH-API-6 | The SerpDad draft target publishes a finished article as a linted draft in under 5 seconds and returns the lint result | End-to-end test | 7 |
| KH-API-7 | Importers for WordPress (WXR and REST, carrying Yoast, Rank Math, AIOSEO, SEOPress and Redirection data), Ghost, Medium, Substack and markdown folders, each with dry run, kept slugs and a redirect map | A 500-post WordPress import loses nothing | 6 |

### 5.9 Modes, operations, performance

| ID | Requirement | Acceptance | Phase |
|---|---|---|---|
| KH-OPS-1 | Server mode and static mode; in static mode every server feature shows a lock card with the feature, one plain sentence why, and the command to switch | Lock text fixtures | 5 |
| KH-OPS-2 | `khalid doctor`, `backup`, `update --rollback`, `theme check`, `pull` for git mode, `keys` | CLI tests | 1, 3, 5 |
| KH-OPS-3 | Every command has `--json`; exit codes 0, 1, 2, 3; no prompt when stdin is not a terminal | CLI tests | 1 |
| KH-OPS-4 | Budgets as CI gates: binary under 30 MB, cold start under 100 ms, idle under 50 MB, post render p50 under 1 ms and p99 under 5 ms, 10,000 requests per second cached on 2 vCPU, full rebuild of 10,000 posts under 30 s, lint under 20 ms, audit of 1,000 posts under 60 s | `perf/baseline.json` and the gate | 1, 2 |
| KH-OPS-5 | Security: sanitised HTML, CSP with nonces, Argon2id, constant-time tokens, path containment, rate limits, content-sniffed uploads, signed webhooks, an outbound client that refuses private networks, graceful shutdown | Tests per rule | 1 to 5 |
| KH-OPS-6 | Releases built by CI from a tag for Linux x86_64 and aarch64, macOS and Windows, with checksums | A prerelease tag produces a draft release with four binaries | 0, 7 |

## 6. Success metrics at 1.0

| Metric | Target |
|---|---|
| Install to first published post, README only | Under 10 minutes |
| Lighthouse, default post page | 100 in all four categories |
| Ahrefs and Semrush Site Audit on the demo site | Health 100, zero errors, zero warnings |
| Theme sync on ten real sites | Eight look native with no edits |
| WordPress import of 500 posts | Zero lost posts, every slug kept, complete redirect map |
| SerpDad's own blog | Live on Khalid at 1.0 |
| Publish from SerpDad | A linted draft in under 5 seconds |
| Every lint rule | A test, a plain message with three parts, a guide link |

## 7. Release plan

Phases 0 to 7 in `docs/plan.md` section 10 build the requirements above in dependency order: foundations and API, then the linter, schema and audit, then theme sync, then the admin, then server features and static mode, then importers, then the SerpDad integration and launch. Phase 8 holds what waits: header and footer cloning, Webflow and HubSpot imports, auto-posting and the social web, cross-posting, paid memberships, Mermaid, Releases, visual editing, a theme directory, agency features, GraphQL.

## 8. Decisions made and where they live

Founder decisions: `docs/plan.md` sections 1 and 12. CTO decisions: `docs/decisions/`. Architecture: `docs/adr/`. Anything not written in one of those three places is open, and open questions go under "Questions for the founder" in the current phase brief.

## 9. Risks to the requirements

| Risk | Mitigation |
|---|---|
| Scope grows past 1.0 | Section 3 is the fence; new asks go to Phase 8 |
| Theme sync fails on JavaScript-rendered sites | Clear message, `--from-css`, clean default; hosted rendering later |
| The editor loses formatting in markdown | A defined block set with directives and round-trip tests |
| AGPL scares companies | README FAQ; unmodified use owes nothing |
| The name | Khalid, chosen by the founder; a trademark lawyer checks before launch |
| One builder, one laptop | A second machine runs parallel tracks when available; CI is the referee |
