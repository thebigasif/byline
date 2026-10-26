# Rust in this repository

The `rust-best-practices` skill holds the general rules. This file holds the Khalid-specific shape: how the workspace is laid out, which crates we use, and the handful of patterns every crate shares. Rule IDs refer to `RULES.md`.

## Workspace

One Cargo workspace at the repository root, flat `crates/` folder, one purpose per crate (BL-SIZE-2). Start with few crates and split when one grows a second purpose. The intended map, as a guide rather than a mandate:

| Crate | Owns | Must not know about |
|---|---|---|
| `khalid` | The binary: `clap` commands, the user-message edge, wiring | Business rules |
| `khalid-core` | Shared newtypes (`Slug`, `SiteId`, `Locale`, `AbsoluteUrl`, `SitePath`), `Clock`, `Config`, `UserMessage`, `Secret` | HTTP, SQL, files |
| `khalid-content` | Front matter, markdown, directives, wikilinks, the content tree, file watching | Templates, HTTP |
| `khalid-routes` | `routes.toml`: collections and permalinks, channels, taxonomy prefixes, aliases, and the URLs they make | Content files, templates, HTTP |
| `khalid-render` | MiniJinja templates, the asset pipeline, the page cache | SQL, HTTP clients |
| `khalid-seo` | Meta tags, schema graphs, sitemaps, feeds, robots, hreflang, redirects | Templates |
| `khalid-share-image` | The generated 1200 by 630 share image and its cache by content key (ADR 0016) | Content files, templates, HTTP |
| `khalid-lint` | The rule engine, every rule, every message. Public, reused by SerpDad | Files on disk, HTTP |
| `khalid-audit` | The self-crawl and the health score | Rendering |
| `khalid-theme` | Token extraction, contrast fixes, proxy rule generation | Content |
| `khalid-db` | SQLite, migrations, typed queries | Rendering |
| `khalid-media` | Image processing, storage backends | HTTP routes |
| `khalid-outbound` | The one HTTP client with the private-network guard (BL-RUN-8) | Everything else |
| `khalid-webhooks` | The event catalogue, signed deliveries, retries, the delivery log model, the verifier for incoming webhooks | Content files, templates, SQL |
| `khalid-server` | Axum routes, the Content and Admin APIs, auth, the embedded admin | Markdown parsing |
| `khalid-import` | One module per source: WordPress, Ghost, Medium, Substack, markdown folders | Rendering |
| `khalid-api` | The Content and Admin API routes, the list query grammar, keys and scopes, problem details, the OpenAPI document. `khalid-server` mounts it | Content files, markdown, SQL, templates |
| `khalid-mcp` | The MCP server, a client of the same functions the API calls | SQL |
| `lint-check` | One deliberate violation per lint, proving the lint table fires (BL-LNT-3) | The product |

Dependencies point downwards in this table. `khalid-core` depends on nothing of ours.

## Crates we use

Choose from this list first. Adding another needs the sentence BL-DEP-1 asks for and a green `cargo deny`.

| Need | Crate |
|---|---|
| Async runtime, HTTP server | `tokio`, `axum`, `tower-http` |
| Database | `sqlx` with the `sqlite` feature, compile-time checked queries |
| Templates | `minijinja`, autoescape on, sandboxed |
| Markdown | `comrak` for parsing with footnotes, tables and task lists; our own directive layer on top |
| HTML | `lol_html` for streaming rewrites, `scraper` for theme sync, `ammonia` for sanitising |
| Serialisation | `serde`, `serde_json`, `toml` |
| Errors | `thiserror` in libraries. `anyhow` only in `crates/khalid` and only to wrap at the very top |
| CLI | `clap` with derive |
| Logging | `tracing`, `tracing-subscriber` |
| HTTP client | `reqwest` with `rustls-tls`, wrapped once in `khalid-outbound` |
| Images | `image`, `ravif`, `webp`, `fast_image_resize` |
| Syntax colours | `syntect` |
| Maths | `latex2mathml` |
| Time | `chrono` behind the `Clock` trait |
| TLS and certificates | `rustls`, `rustls-acme` |
| Passwords and tokens | `argon2`, `rand`, `subtle`, `hmac`, `sha2` |
| Passkeys | `webauthn-rs` |
| Email | `lettre` |
| Embedded admin | `rust-embed` |
| OpenAPI | `utoipa`, `utoipa-axum` |
| File watching | `notify` |
| Ordered maps | `indexmap` |
| Allocator | `mimalloc` in the binary |
| Tests | `insta`, `proptest`, `wiremock`, `tempfile`, `assert_cmd` |
| Benchmarks | `divan` |

## Shared patterns

**Clock (BL-RUN-1).** `khalid_core::Clock` is a trait with `now()`. The real clock is one struct. Tests pass a `FixedClock`. `clippy.toml` disallows the direct `now` methods everywhere else.

**Config (BL-RUN-2).** `khalid_core::Config` is read once from `site.toml`, environment and flags, validated, then passed by reference or `Arc`. No module reads the environment on its own.

**UserMessage (BL-ERR-4).** Three fields, `what`, `why`, `fix`, each one plain sentence. Library errors implement a `ToUserMessage` trait in the binary crate, never inside the library. The same struct serialises into the API's problem details and the CLI's `--json` output.

**Newtypes (BL-TYP-3).** `Slug::try_new(&str)`, `SitePath::within(site_root, untrusted)`, `AbsoluteUrl::parse`. Each validates once. A function that takes one of these cannot receive junk.

**Outbound (BL-RUN-8).** One `Outbound` client. It resolves DNS, refuses private ranges, caps response size at a configured limit, caps redirects, and sets timeouts. Every crate that talks to the outside takes an `&Outbound`.

**Secret (BL-RUN-5).** `Secret<T>` whose `Debug` and `Display` print `[redacted]`. Tokens, keys and passwords are never bare strings past the edge.

**Ordered output (BL-TYP-5).** Anything that lands in a page, a feed, a sitemap or a file iterates a `Vec`, a `BTreeMap` or an `IndexMap`.

## Async rules

- `async fn` everywhere a function awaits. No hand-written futures without a reason.
- CPU-heavy work goes to `spawn_blocking` or a bounded worker pool: image encoding, large imports, full rebuilds (BL-RUN-7).
- No lock across `.await` (BL-RUN-6). Prefer channels and single-owner tasks to shared mutable state.
- Every spawned task has a name through `tokio::task::Builder` where available, so a hang can be seen.

## Errors in practice

```rust
#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    #[error("front matter in {path} is missing the `date` field")]
    MissingDate { path: PathBuf },
    #[error("could not read {path}")]
    Read { path: PathBuf, #[source] source: std::io::Error },
}
```

The binary maps `ContentError::MissingDate` to a `UserMessage`: "This post has no date. Google and readers use dates to judge freshness. Add `date:` to the header of `posts/why-links-matter.md`."

## Build speed

A product that is fast to build ships faster.

- `cargo check` before `cargo build`. `cargo nextest` for tests. `sccache` and the `mold` linker in CI.
- Thin LTO, `codegen-units = 1` and `strip = "symbols"` in release. Debug builds carry `debug = "line-tables-only"`.
- Heavy dependencies are weighed with `cargo bloat --release --crates` in CI, and the top ten are printed in the job summary.
- Feature flags are bare and additive. The binary enables what it ships; libraries enable nothing by default that pulls a heavy tree.

## Adding a crate

1. `cargo new --lib crates/khalid-<name>` with `[lints] workspace = true`.
2. Write the `//!` crate docs first: what it owns, what it must not know about (BL-DOC-2).
3. Add it to the table above in the same pull request.
