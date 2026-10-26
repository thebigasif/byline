# 0009. Render input records and the template sandbox

**Status:** Accepted, 2026-10-26. Phase 1 task 1.11.

## Context

`khalid-render` turns pages into HTML. The content crate, which knows posts, is built at the same time by another session, and the two meet only when the server is wired in task 1.15. Templates can be replaced by the site owner from `theme/templates/`, so they are code we did not write running inside the server.

## Decision

- **Own input records.** The render crate takes its own records (`RenderSite`, `PageMeta`, `PageBody` and the page records that follow) and does not depend on `khalid-content`. The wiring code fills them. Templates see only what these records hold.
- **Exhaustive on purpose.** The records have public fields and are not `#[non_exhaustive]`, which RULES.md BL-API-6 allows for a stable set. Here the reason is stronger: a new field must stop the build of the wiring code until it fills the field, rather than render an empty value. `PageBody`, the list of page types, is `#[non_exhaustive]`, because adding a page type must not break a caller that matches on it.
- **The sandbox (BL-SEC-10).** MiniJinja runs without a loader, so `include` and `extends` reach only the built-in templates and the theme's replacements, read once at start through `SitePath`. No function reads files, the network or the clock. Every value is escaped for HTML unless the crate marks it safe. A missing value is an error, not an empty string. Each page has a fuel budget, so a theme template that loops without end stops with an error instead of holding a worker.
- **Only known names.** A file in `theme/templates/` must replace a built-in template of the same name. Any other file is refused with its name, so a typo cannot silently do nothing.
- **Escaping.** The crate escapes `&`, `<`, `>`, `"` and `'`, which is all HTML needs. MiniJinja's own escaper also writes every `/` as `&#x2f;`, which made every address in every page longer.
- **Built in, always.** Templates and strings are built into the binary in debug builds too (`rust-embed` with `debug-embed`), so development renders exactly what production renders (survey lesson 1).

## Consequences

- Task 1.15 writes the mapping from content to these records. It is the one place that knows both crates.
- A theme that needs a value the records do not hold asks for a field; it cannot reach around the records.
- Theme partials with new names are not supported yet. When they are, they get their own folder and their own rule.
