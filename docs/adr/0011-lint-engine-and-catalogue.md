# 0011. The lint engine reads its own input model and one message catalogue

**Status:** Accepted, 2026-10-26. Phase 2 work pulled forward, task 2.1.

## Context

`crates/khalid-lint` holds every rule from plan section 6. SerpDad's backend depends on it to lint its own drafts (KH-LNT-6), so it cannot read Khalid's content files, database or templates. The content crate that parses posts is still being built by another session. Every message must say what, why and fix (ADR 0004), and the same words must reach the command line, the API and the admin.

## Decision

- The crate defines its own input: `LintPost` for a post and `LintSite` for the site, read by rules through the `SiteView` trait. It depends on `khalid-core` only, for `Locale` and `Slug`. Task 1.15 implements `SiteView` for the content track's types.
- A rule only finds problems. It returns hits; the engine adds the severity from `lint.toml` or the rule's default, and the message from the catalogue. A rule cannot decide on its own to block a post.
- Messages live in one file, `crates/khalid-lint/catalogue.toml`, keyed by rule ID, built into the crate. Each has `what`, `why`, `fix` and a `guide` slug that the docs map to a SerpDad guide later. A linter refuses to start with a rule that has no message.
- `lint.toml` has one table, `[rules]`, mapping a rule ID to `block`, `warn` or `off`. An unknown key, rule or severity is an error naming the line and column.
- Rule IDs are kebab-case and never change once released, because sites name them in `lint.toml`.

## Consequences

- The linter is tested on its own, with fixtures written as input values, and SerpDad can use it without the rest of Khalid.
- One test walks the catalogue and checks every message, so a new rule cannot ship with a weak message.
- Task 1.15 carries the mapping from parsed posts to `LintPost`. That mapping is the one place to change when content types change.
