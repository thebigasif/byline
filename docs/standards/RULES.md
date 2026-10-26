# Rule catalogue

One file is the standard. The skills point to it, reviewers cite its IDs, and every lint in `lints/lints.toml` maps to a rule here. No rule enters this file without an ID, a source and an enforcement.

**Sources**

| Code | Source |
|---|---|
| BR | BeautifulRust, https://github.com/morganlinton/BeautifulRust |
| API | Rust API Guidelines |
| ER | Effective Rust |
| MS | Microsoft Pragmatic Rust Guidelines |
| CL | clippy documentation |
| OWASP | OWASP ASVS and cheat sheets |
| PLAN | a founder decision in `docs/plan.md` |
| BL | a Khalid engineering decision, recorded in `docs/adr/` |

**Enforcement**

| Code | Meaning |
|---|---|
| L | a compiler or clippy lint, proven to fire by `lints/check` |
| C | `clippy.toml` `disallowed-methods` or `disallowed-types` |
| S | a CI script or gate |
| T | a required test, checked in review |
| R | review only, with the `khalid-review` skill |

## Errors

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-ERR-1 | Each library crate has its own error enum built with `thiserror`. No `Result<_, String>`, no `Box<dyn Error>`, no `anyhow` in a library crate | ER, API, BR | L (`result_large_err`), S (search for `anyhow` outside `crates/khalid`) |
| BL-ERR-2 | Keep the cause: `#[from]` or `#[source]`, never `e.to_string()` into a variant | ER | R |
| BL-ERR-3 | No panics outside tests: no `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `unreachable!`, slice indexing or integer `as` casts | BR, CL | L (`unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, `indexing_slicing`, `as_conversions`) |
| BL-ERR-4 | Every error that reaches a person becomes a `UserMessage` with three fields: what happened, why it matters, what to do. Produced at the edge in the binary and the API, never inside a library | PLAN, BL | T, R |
| BL-ERR-5 | API errors are RFC 9457 problem details: `type`, `title`, `status`, `detail`, plus `fix` and `rule` where a lint or validation produced them | BL | T |
| BL-ERR-6 | A fallible function that consumes an argument returns it inside the error so the caller can retry | BR | R |

## Types

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-TYP-1 | A status, kind, state or mode is an enum. No stringly typed states, no `bool` flags that mean a mode | BR, ER | L (`fn_params_excessive_bools`, `struct_excessive_bools`), R |
| BL-TYP-2 | `serde_json::Value` only at the edges: the API, stored JSON, outside services | BR, BL | S (count ratchet) |
| BL-TYP-3 | Identifiers and validated strings are newtypes with fallible constructors: `Slug`, `SiteId`, `PostId`, `Locale`, `AbsoluteUrl`, `SitePath`, `AnchorText`. Never a bare `String` for one of these in a signature | BR, API | R |
| BL-TYP-4 | Number casts go through `From` or `TryFrom`. An intended lossy cast has an `#[expect]` with the reason | BR, CL | L (`as_conversions`, `cast_possible_truncation`, `cast_sign_loss`) |
| BL-TYP-5 | Where order reaches a page, a file, a feed, a sitemap or a test, use `Vec`, `BTreeMap` or `IndexMap`. Never iterate a `HashMap` into output | BR | C (`disallowed-types` for `HashMap` in render and seo crates), R |
| BL-TYP-6 | Money is integer minor units. Durations are `Duration`. Paths are `Path` and `PathBuf`. Times are UTC in storage and localised only in templates | BR, BL | R |

## API shape

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-API-1 | No `bool` parameters on public or crate-public functions. Use a two-variant enum or two functions | BR, API | L (`fn_params_excessive_bools` with `max-fn-params-bools = 0`) |
| BL-API-2 | Take `&str`, `&[T]`, `&Path`, `&T`. Take ownership only when storing | BR, API | L (`ptr_arg`, `needless_pass_by_value`) |
| BL-API-3 | No `get_` getters. `as_`, `to_` and `into_` follow their cost and ownership. Constructors are `new`, `try_new`, `from_x`. Error types read verb-object-error | API | R |
| BL-API-4 | Every HTTP endpoint is declared in the OpenAPI spec through `utoipa` annotations before it is implemented, carries `Accept-Version`, and lives under `/api/content/v1/` or `/api/admin/v1/` | PLAN, BL | T (spec contract test), R |
| BL-API-5 | List endpoints share one query grammar: `filter`, `include`, `fields`, `formats`, `limit`, `page`, `order`, parsed by one crate and tested once | PLAN | T |
| BL-API-6 | Public enums and public-field structs that cross a crate boundary are `#[non_exhaustive]` unless the set is a stable ontology | BR, API | R |
| BL-API-7 | Every mutating endpoint is idempotent or takes an idempotency key. Edits carry `updated_at` and a stale edit returns 409 | PLAN, BL | T |

## Runtime

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-RUN-1 | Time is read through the `Clock` trait. No `SystemTime::now`, `Utc::now` or `Instant::now` outside the clock module | BL | C (`disallowed-methods`) |
| BL-RUN-2 | Settings are read once into a typed `Config` at start. No `std::env::var` outside the config module | BL | C |
| BL-RUN-3 | SQL never contains a runtime value. Bind every value. `format!` may join only `const` fragments | OWASP, BL | S (search check), R |
| BL-RUN-4 | Output goes through `tracing`. No `println!` or `eprintln!` outside the binary's command output | BR | L (`print_stdout`, `print_stderr`) |
| BL-RUN-5 | Never log a secret, token, password, key, session or personal email. Wrap such values in `Secret<T>` whose `Debug` redacts | OWASP, BL | T, R |
| BL-RUN-6 | No lock held across `.await`. No `MutexGuard` returned from a public function | CL, BR | L (`await_holding_lock`), R |
| BL-RUN-7 | No blocking I/O and no CPU-heavy work on the async runtime. Image work, markdown rendering of large files and imports run in `spawn_blocking` or a worker pool | BR, BL | R, S (benchmark gate) |
| BL-RUN-8 | Every outbound HTTP request goes through the one `Outbound` client: timeouts, response size cap, redirect cap, and a guard that refuses private, loopback and link-local addresses after DNS resolution. Theme sync, bookmarks, imports, webhooks and IndexNow all use it | OWASP, BL | C (`disallowed-types` for raw `reqwest::Client` outside the outbound module), T |
| BL-RUN-9 | Writes under `content/`, `media/` and `theme/` are atomic: write to a temp file in the same directory, fsync, rename | BL | R, T |
| BL-RUN-10 | The server shuts down gracefully: stop accepting, finish in-flight requests, flush the queue, close SQLite | BL | T |

## Security

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-SEC-1 | HTML that came from a person is sanitised with `ammonia` against an allowlist before storage and again before render. Markdown rendering escapes raw HTML unless the block is the HTML block, which is sanitised | OWASP | T |
| BL-SEC-2 | Every response carries a Content Security Policy with a per-request nonce. No inline event handlers. No `unsafe-inline` for scripts | OWASP, PLAN | T (header test on every page type) |
| BL-SEC-3 | Passwords hash with Argon2id. Tokens are 256 random bits, stored hashed, compared in constant time. Sessions are signed cookies, `HttpOnly`, `Secure`, `SameSite=Lax` | OWASP | T |
| BL-SEC-4 | Every path from a request or a config becomes a `SitePath` that cannot escape the site folder. No `Path::join` on untrusted input outside that type | OWASP, BL | C (`disallowed-methods` for `Path::join` in server crates), T |
| BL-SEC-5 | Rate limits on sign-in, forms, comments, search and the API, per IP and per key, with a plain message when hit | OWASP | T |
| BL-SEC-6 | Uploads are type-checked by content, not extension. Images are re-encoded. SVG is accepted only after sanitising. Executable types are refused | OWASP | T |
| BL-SEC-7 | Webhook deliveries are signed with HMAC-SHA256 over timestamp and body. Incoming webhooks verify the same way | PLAN, BL | T |
| BL-SEC-8 | `cargo deny` passes: no unpatched advisory, no banned crate, no unknown licence | BR | S |
| BL-SEC-9 | Admin forms carry a CSRF token and the API refuses cookie auth without the `X-Requested-With` header or a token | OWASP | T |
| BL-SEC-10 | Templates render with autoescape on in a sandboxed MiniJinja environment. No template can read the file system or call arbitrary functions | OWASP, BL | T |

## Web

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-WEB-1 | Every page is semantic HTML: `header`, `nav`, `main`, `article`, `aside`, `footer`, `time`, `figure`, exactly one `h1`, headings in order, `lang` and `dir` set | PLAN | T (validator and axe in CI) |
| BL-WEB-2 | Every public page reads and every public form works with scripts off. JavaScript only adds polish | PLAN | T (scripts-off run in CI) |
| BL-WEB-3 | No horizontal scroll from 320 px to 1920 px wide and at 200% zoom. Touch targets at least 44 px | PLAN | T (six widths in CI) |
| BL-WEB-4 | WCAG 2.2 AA. Contrast is checked against the synced tokens | PLAN | T (axe, contrast check) |
| BL-WEB-5 | Zero layout shift: every image has `width`, `height`, `srcset`, `sizes` and `alt`; fonts have tuned fallbacks | PLAN | T (Lighthouse CLS) |
| BL-WEB-6 | No third-party script, font or style by default. Anything external is opt-in in settings and allowed through the CSP | PLAN | T |
| BL-WEB-7 | One CSS file under 20 KB compressed. No CSS framework on the public site | PLAN | S (size gate) |

## Command line

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-CLI-1 | Every command accepts `--json` and prints machine-readable output with the same fields the API uses | PLAN | T |
| BL-CLI-2 | Exit codes: 0 success, 1 a block or a failure the user must act on, 2 usage error, 3 an internal fault | BL | T |
| BL-CLI-3 | Every message says what happened, why it matters, what to do, as three short sentences | PLAN | T (message fixtures) |
| BL-CLI-4 | No interactive prompt when `--yes` is given or stdin is not a terminal | BL | T |

## Files and content

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-FS-1 | Files under `content/` are the source of truth. The database is rebuilt from them on demand and holds only derived or visitor data | PLAN | R |
| BL-FS-2 | Nothing derived is written under `content/`. Lint results, link graphs, caches and indexes live in `data/` | PLAN | T |
| BL-FS-3 | Front matter is validated against a typed schema. An error names the file, the line and the field, in plain words | PLAN | T |
| BL-FS-4 | A file change rebuilds only what depends on it. A full rebuild of 10,000 posts stays under 30 seconds | PLAN, BL | S (benchmark) |
| BL-FS-5 | Every editor block has a markdown directive, and parse then print returns the same text | PLAN | T (round-trip test per block) |

## Database

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-DB-1 | SQLite in WAL mode, foreign keys on, a busy timeout, one writer connection and a read pool | BL | T |
| BL-DB-2 | Schema changes are forward-only migrations through `sqlx::migrate!`. A migration never edits an earlier one | BL | S |
| BL-DB-3 | Queries are checked at compile time with the `sqlx` macros, or built with bound parameters. No SQL assembled from runtime strings | OWASP, BL | S, R |

## Performance

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-PERF-1 | The budgets in `docs/standards/performance.md` are CI gates. A regression over the allowed drift fails the build | PLAN | S |
| BL-PERF-2 | The render path does no network calls and no disk reads except on a cache miss. Everything a page needs is computed at publish time | PLAN, BL | R, S (benchmark) |
| BL-PERF-3 | Hot paths reuse buffers and use `Bytes` and `Arc<str>` for shared immutable data. No per-request allocation storms | BR | R, S (allocation count in the render benchmark) |
| BL-PERF-4 | Any change under `crates/*render*`, `crates/*lint*` or `crates/*server*` runs the benchmark suite and reports before and after in the pull request | BL | S |
| BL-PERF-5 | Images are processed at upload and publish, never at request time | PLAN | R |

## Lints and suppressions

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-LNT-1 | Every suppression is `#[expect(lint, reason = "...")]` on the narrowest item. Never `#[allow]`, except one file-wide `#![expect]` on generated code naming the generator | BR | L (`allow_attributes`, `allow_attributes_without_reason`) |
| BL-LNT-2 | Never `#![deny(warnings)]` in source. CI owns warnings as errors | BR | S |
| BL-LNT-3 | Every lint in `lints/lints.toml` has one deliberate violation in `lints/check`, and `scripts/check-lints.sh` proves it fires | BR | S |

## Size and structure

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-SIZE-1 | Functions under about 80 lines and six parameters. Files under about 600 hand-written lines | BR, CL, BL | L (`too_many_lines` at 80, `too_many_arguments` at 6), S (file size) |
| BL-SIZE-2 | A crate does one thing and is named for it. Split when it grows a second purpose. Never nest a crate in another crate's `src/` | BR | R |
| BL-SIZE-3 | Leaf modules are named after their main item. Non-leaf modules hold only `mod` lines and re-exports | BR | R |

## Tests

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-TST-1 | A failing test comes before the fix or the feature | BR, BL | R |
| BL-TST-2 | Tests are deterministic: a fixed clock, a seeded random source, no dependence on run order or `HashMap` order | BL | R |
| BL-TST-3 | No test touches the network. Outside services are stubbed with `wiremock` | BL | S (network disabled in CI test job) |
| BL-TST-4 | Every lint rule has one fixture per message it can emit, and a test that the message has all three parts | PLAN | T |
| BL-TST-5 | Every editor block has a round-trip test and a rendered-HTML snapshot | PLAN | T |
| BL-TST-6 | Line coverage stays at or above the committed floor, which only ratchets up | BL | S (`cargo llvm-cov`) |
| BL-TST-7 | Rendered HTML is snapshot-tested with `insta`. A snapshot change is reviewed as a visible change | BL | T |
| BL-TST-8 | Anything that does I/O, time or randomness is mockable behind a trait with a `test-util` feature, never a `Default` that performs I/O | BR | R |

## Documentation

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-DOC-1 | Every public item has docs: a one-line summary, then `# Examples`, `# Errors`, `# Panics` as they apply. Examples use `?`, never `unwrap` | BR, API | L (`missing_docs`, `missing_errors_doc`, `missing_panics_doc`), S (`cargo doc` with warnings as errors) |
| BL-DOC-2 | Every crate root has `//!` docs that say what the crate owns and what it must not know about | BR, BL | R |
| BL-DOC-3 | Every decision the plan does not cover gets an ADR in `docs/adr/`, numbered, with context, decision and consequences | BL | R |
| BL-DOC-4 | All text a person reads is plain language: short sentences, one idea each, terms defined on first use, no em dashes | PLAN | S (`typos` and a dash check), R |

## Dependencies

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-DEP-1 | A new dependency needs one sentence in the pull request: what it does that std and the crates we have do not | BR | R |
| BL-DEP-2 | Permissive licences only, so SerpDad can reuse every crate in its closed backend. MPL-2.0 by named exception. No GPL-family dependency | PLAN, BL | S (`cargo deny`) |
| BL-DEP-3 | One version of each crate where possible. A duplicate needs a reason in `deny.toml` | BR | S |
| BL-DEP-4 | The toolchain is pinned in `rust-toolchain.toml` and raised in its own pull request | BR | S |
| BL-DEP-5 | Version ranges, never `*` and never a needless `=` | BR | R |

## Text and language

| ID | Rule | Source | Enforced |
|---|---|---|---|
| BL-TXT-1 | No em dashes in any file in this repository | PLAN | S (`scripts/check.sh` dash check) |
| BL-TXT-2 | Every lint message, error message and lock card has the three parts: what, why, fix | PLAN | T |
| BL-TXT-3 | Public-site strings live in one strings file per language. Templates never hard-code a user-visible word | PLAN | R |
