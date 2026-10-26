# Phase 0: name, repository, rules

**Goal:** a repository people could star, with the tools that keep the code at 10/10 switched on before the first line of product code. Plan section 10, Phase 0.

**Already done by the planning session:** this repository, the licence, the CLA text and bot workflow, the trademark note, the code of conduct, `CLAUDE.md`, the rule catalogue, the standards, the skills, the lint table, the workspace `Cargo.toml`, the pinned toolchain, `deny.toml`, the CI workflows, the issue and pull request templates, the plan and the ADRs.

## Tasks

### 0.1 Name checks (founder decides, agent prepares)

- Status: `todo`
- For each name on the shortlist in `docs/plan.md` D-1 (Byline, Masthead, Dateline, Gazette, Typeset), check and record: GitHub organisation and repository name free, crates.io name free, npm name free, `.dev` and `.com` domains, a USPTO and EUIPO trademark search for classes 9 and 42, and whether any CMS, editor or publishing tool already uses the name.
- Write the results as a table in `docs/decisions/name.md` with links. Make no recommendation beyond the plan's. The founder picks.
- Done when: the table has every check for every name and the founder has replied.

### 0.2 The first crate: `byline` prints help

- Status: `doing`. Split into stacked pull requests to stay under 400 lines each: the core crate and a binary that prints help, then `Slug`, then the commands, then `--json`, then `Clock`, `Config` and `Secret`.
- `crates/byline` with `clap`, the commands from plan section 4.1 as subcommands that print "not built yet" with exit code 3, `--version`, `--json` on every command, the `UserMessage` edge, and `mimalloc`.
- `crates/byline-core` with `Clock`, `FixedClock`, `Config` (empty for now), `UserMessage`, `Secret<T>`, and the `Slug` newtype with its tests. Crate docs that say what each owns.
- Done when: `cargo run -p byline -- --help` prints the commands, `just check` is green, CI is green, the binary is under 5 MB.

### 0.3 The lint proof crate

- Status: `doing`
- `lints/check` with one deliberate violation per lint in `lints/lints.toml` set to warn, marked `// expect: <lint>`, following `lints/check-reference/`. Adapt `scripts/check-lints.sh` to the workspace layout. Delete `lints/check-reference/`.
- Done when: `./scripts/check-lints.sh` passes, and removing any one lint from the table makes it fail.

### 0.4 Disallowed methods and types carry their reasons

- Status: `todo`
- The clock module, the config module and the outbound module each carry one `#[expect(clippy::disallowed_methods, reason = "...")]` or `disallowed_types` on the one line that is allowed to break the rule. A test in `lints/check` proves the disallowed methods fire elsewhere.
- Done when: `cargo clippy` is clean and the expects are the only ones in the workspace.

### 0.5 The demo fixture site

- Status: `todo`
- `fixtures/demo-site/` with `site.toml`, five real posts about SEO in plain English (no lorem ipsum), two authors, three categories, one glossary term, in English and German. This is the fixture every benchmark, snapshot and end-to-end test uses from Phase 1.
- Done when: the folder exists, the posts read like real posts, and a test asserts the fixture parses once Phase 1 lands.

### 0.6 Performance gate skeleton

- Status: `todo`
- `perf/baseline.json` with the budgets from `docs/standards/performance.md` as the first baseline, `scripts/perf.sh` measuring binary size and cold start of `byline --help`, and `scripts/perf-gate.py` comparing with the allowed drift.
- Done when: the perf job in CI runs the two measures and fails when the binary is made artificially large in a test branch.

### 0.7 CHANGELOG and release workflow

- Status: `todo`
- `CHANGELOG.md` with an Unreleased section. `.github/workflows/release.yml` that builds the binary for Linux x86_64 and aarch64, macOS and Windows on a `v*` tag, strips it, attaches it to a GitHub release with checksums, and refuses to run when `just check` fails.
- Done when: a `v0.0.1` tag on a test branch produces a draft release with four binaries.

## Questions for the founder

- **Accented slugs (0.2, not blocking).** A slug today is lowercase letters a to z, digits and hyphens, so `über-seo` is refused and must be written `uber-seo`. That keeps addresses identical on every server and easy to type. Should the German site be allowed slugs with accents such as `ü`? The default stays a to z until you say otherwise.

## Order

0.3 first, then 0.2. CI runs `scripts/check-lints.sh` as soon as the first crate exists, so the lint proof crate has to land before the first product crate for CI to go green. Then 0.4, then 0.6, 0.5, 0.7. 0.1 runs in parallel and does not block anything.
