# Testing

Tests are the proof the founder asked for. A feature without a test is not done.

## Kinds, and when each is required

| Kind | Where | Required for |
|---|---|---|
| Unit | `#[cfg(test)] mod tests` beside the code | Every function with a branch |
| Integration | `crates/<crate>/tests/` | Every public API of a crate |
| Doc tests | rustdoc examples | Every public function with an example, using `?` |
| Property | `proptest` | Parsers, slug and URL rules, the filter grammar, directive round-trips |
| Snapshot | `insta` | Rendered HTML per template and per block, schema JSON, sitemaps, feeds |
| Lint fixtures | `crates/byline-lint/fixtures/` | One fixture per message a rule can emit, and a check that each message has what, why and fix (BL-TST-4) |
| Round-trip | `crates/byline-content/tests/` | Every block: markdown to blocks to markdown, byte for byte (BL-TST-5) |
| HTTP | `axum` test client | Every endpoint: success, validation failure, auth failure, stale `updated_at` |
| CLI | `assert_cmd` | Every command: human output, `--json`, exit code |
| End to end | Playwright in `e2e/` | Every user journey in `docs/plan.md` section 2, at six widths, with and without scripts |
| Accessibility | axe in Playwright | Every page type and admin screen |
| Benchmarks | `divan` in `benches/` | Render, lint, rebuild, audit |

## Rules

- **Failing test first** (BL-TST-1).
- **Deterministic** (BL-TST-2). A `FixedClock`, a seeded random source, ordered collections. A flaky test is a bug with priority over the feature.
- **No network** (BL-TST-3). `wiremock` stands in for Google, Unsplash, IndexNow, webhook receivers and the sites theme sync reads. The CI test job runs with the network disabled.
- **Name the behaviour.** `blocks_a_post_with_no_author`, not `test_author`. Assertions carry context: `assert_eq!(got, want, "post {path}")`.
- **Fixtures are small and real.** A fixture is the smallest content that shows the behaviour, with real words, never `lorem ipsum` or `foo`.
- **Coverage floor** (BL-TST-6). `cargo llvm-cov` reports line coverage. The floor in `coverage.toml` starts at 80% and only moves up.
- **Snapshots are reviewed** (BL-TST-7). A changed snapshot is a visible change. The pull request says why it changed.

## The messages test

Every lint message, CLI message, API problem detail and lock card has three parts. One test walks every message in the catalogue and checks: three sentences, each under 25 words, no em dash, no jargon word from the banned list, and the fix names an action.
