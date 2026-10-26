# What other Rust CMSs and site generators teach us

Written 2026-10-26 by the CTO session at the founder's request: "learn from other Rust CMS already built on GitHub". Twelve projects were read, their READMEs, dependency lists and crate layouts. Each row says what to copy and what to avoid. The lessons at the end are instructions for Phase 1 and later, with the task they belong to.

## The projects

| Project | Stars | Licence | What it is | Copy | Avoid |
|---|---|---|---|---|---|
| [getzola/zola](https://github.com/getzola/zola) | 17,556 | MIT | The Rust static site generator. One binary, Tera templates, sections and pages, taxonomies, co-located assets, link checker, search index, live reload | The content vocabulary (sections with `_index.md`, pages, taxonomies). The `components/` split: config, content, markdown, render, templates, search, link_checker, imageproc. The `test_site` and `test_sites_invalid` fixtures. An RFC folder for design changes | Static only, no editor, no server features; that gap is our product |
| [zensical/zensical](https://github.com/zensical/zensical) | 6,003 | MIT | Docs generator by the Material for MkDocs team, Rust core | Search in 60 languages, polished defaults, a paid tier beside a free core | Documentation focus, not blogs |
| [rochacbruno/marmite](https://github.com/rochacbruno/marmite) | 887 | AGPL-3.0 | Zero-config blog generator from a folder of markdown, built largely with agents | Wikilinks and backlinks, series, language streams with hreflang, `date:` decides post versus page, parallel image resizing with rayon and incremental rebuilds, `rust-embed` for templates and assets, an `AGENTS.md` and agent skills shipped inside the binary | Regex shortcodes (fragile), a 2,000-line `site.rs` |
| [bearcove/dodeca](https://github.com/bearcove/dodeca) | 225 | Apache-2.0 | Fully incremental generator built on a query system | "Dev mode is production mode": serve exactly the production output while writing. Every transformation a cached query. Cache-busted URLs, font subsetting, responsive images, link checking. An authoring language server | A custom template engine; MiniJinja gives us the same with less to maintain |
| [bearcove/home](https://github.com/bearcove/home) | 132 | none | The CMS behind fasterthanli.me | Ideas in crate names: `libdoctor`, `librevision`, `libhtmlrewrite`, `dumbcache`, `libsearch` | Sixty crates for one site. Our fifteen are plenty |
| [rikhuijzer/bloglite](https://github.com/rikhuijzer/bloglite) | 332 | MIT | Self-hosted microblog, a few megabytes of memory | Tiny footprint as a feature, write and edit from a phone, automatic backup to plain text files | Microblog scope |
| [RaisFast/raisfast](https://github.com/RaisFast/raisfast) | 55 | Apache-2.0 | Single-binary headless CMS plus commerce, wallets, four plugin engines, MCP server | MCP as a first-class interface, React and shadcn admin embedded in the binary, SQLite first | Scope: commerce, wallets, four scripting engines in an alpha. Our v1 has no plugin API on purpose |
| [avored/avored-rust-cms](https://github.com/avored/avored-rust-cms) | 153 | none | Headless CMS with a Leptos admin and embedded SurrealDB | Custom content types, review workflows, Argon2, versioned API | Admin in Rust and WebAssembly ties every UI change to a Rust compile. SurrealDB is a heavy embedded dependency |
| [fermyon/bartholomew](https://github.com/fermyon/bartholomew) | 461 | Apache-2.0 | Micro-CMS compiled to WebAssembly for Spin | Handlebars templates, Rhai scripting, a tiny core | Needs a Spin host; niche |
| [nitn3lav/derived-cms](https://github.com/nitn3lav/derived-cms) | 9 | MPL-2.0 | Generate admin, API and storage from Rust type definitions | The idea: one typed struct drives the form, the API and the table. Our collections (glossary, case studies, changelog) should work this way | Tiny project, one author |
| [prabirshrestha/rblog](https://github.com/prabirshrestha/rblog) | 52 | MIT | Blog engine, posts as files in a folder, Salvo server | Posts as files served live, `notify` for reloads, Docker volume for `posts/` | One author's blog |
| [Rust-CMS/radical](https://github.com/Rust-CMS/radical), [Kilerd/rubble](https://github.com/Kilerd/rubble) | 82, 51 | MIT | WordPress-style engines on MySQL and Postgres with Diesel | Nothing new | Both stopped in 2024 and 2022. An external database made install hard and the projects died. Our SQLite decision stands |

## Lessons for Khalid

Each lesson names the Phase 1 task it changes, or the phase it waits for.

1. **Dev mode is production mode** (dodeca). `khalid serve` in a site folder serves exactly what `khalid build` or production would serve: hashed asset names, minified HTML, responsive images, subset fonts. No shortcuts in development, so what the writer sees is what ships. Tasks 1.11, 1.12, 1.15.
2. **Zola's content vocabulary, with our names.** A folder is a collection, `_index.md` holds the collection's own front matter and body, pages are files or folders with `index.md` and their assets beside them. Writers who know Zola or Hugo feel at home, and migration from them is a copy. Task 1.5.
3. **Fixtures the way Zola does them.** One `fixtures/demo-site` that every test, benchmark and snapshot uses, plus `fixtures/invalid/*` where each folder breaks one rule and carries the exact message the user must see. The invalid fixtures are the test suite for `UserMessage`. Task 1.5 and every task that adds a rule.
4. **Backlinks at build time** (Marmite) are our linked mentions, computed once when content changes and stored in the link graph table, never at request time. Task 1.8 for wikilinks, 1.13 for the output.
5. **Incremental everything** (dodeca, Marmite). Every derived artefact is keyed by the content hash of its inputs: rendered HTML, resized images, search index, sitemaps. A change to one post re-renders that post, its collection page, and the pages that link to it, nothing else. The benchmark for BL-FS-4 measures this. Tasks 1.5, 1.12, 1.21.
6. **A link checker as a component** (Zola). Internal links are checked against the content tree at lint time, external links through the `Outbound` client with a cache and a rate limit, never on every build. Phase 2 lint rules reuse it. Task 1.4 provides the client, Phase 2 the rule.
7. **Search in both modes** (Zola, Marmite). Server mode uses SQLite full-text search. Static mode exports a compact JSON index and a few kilobytes of script, which is what Zola ships. One index builder feeds both. Phase 5.
8. **Ship the agent guide inside the binary** (Marmite embeds its agent skills). `khalid agents` prints the user-facing `AGENTS.md` and the Claude Code skill for running a Khalid site, so an agent on any machine gets the instructions without finding the repository. Task 1.16, small.
9. **Parallel image work** (Marmite with rayon). Resize and encode in a worker pool at upload and publish, never at request time, and cache by input hash. Task 1.12.
10. **Typed collections drive form, API and storage from one definition** (derived-cms). Our glossary, case studies and changelog are one Rust struct each, with the front matter schema, the admin form, the API shape and the database columns derived from it. Task 1.5 for the schema, 1.17 and 1.18 for the API.
11. **Admin in TypeScript, not in Rust** (lesson from Avored). An admin written in Leptos or Yew means every button change is a Rust compile. RaisFast made the other choice, React and shadcn embedded in the binary, and so do we. Phase 4.
12. **SQLite and nothing else** (lesson from radical and rubble, which died, and from bloglite and RaisFast, which live). An external database doubles the install and halves the audience. Already decided; this is the evidence.
13. **No plugin engines in v1** (lesson from RaisFast). Four scripting runtimes in an alpha is scope that never ships. Webhooks and the API are our extension points until users ask for more.
14. **Fifteen crates, not sixty** (lesson from home). Split a crate when it grows a second purpose, not before.
15. **Backups as plain text** (bloglite). Our content is already files, and `khalid backup` is an archive of the folder. Say so on the free tools page, because people fear lock-in.
16. **An authoring language server** (dodeca) is the end state of "CLI ready": directive completion, front matter validation and lint messages inside the writer's editor. Phase 8 candidate, recorded here so it is not forgotten.

## Dependencies they share, which confirms ours

Every project that survived uses `comrak` or `pulldown-cmark` for markdown, Tera or a Jinja-like engine for templates, `clap` for the command line, `notify` for file watching, `rust-embed` or `include_dir` for embedded assets, `image` for pictures, `axum` or Salvo for serving, and `tokio`. Our list in `docs/standards/rust.md` matches, with MiniJinja in place of Tera because it is faster, sandboxed and reloads templates without a restart.
