---
name: khalid-way
description: >-
  The Khalid repository's own rules on top of rust-best-practices: the founder's
  decisions, the rule catalogue IDs, the shared patterns (Clock, Config, UserMessage,
  SitePath, Outbound, Secret), the performance budgets and the definition of done.
  Load this whenever you write, change, review or plan code, docs, templates, CI or
  tests in this repository, even for a one-line change. When this skill and a general
  skill disagree, this skill wins.
---

# The Khalid way

Read `CLAUDE.md` once per session. This skill is the short form you keep in mind while coding.

## Before you write

- Which task in `docs/phases/` is this? If none, stop and ask in the pull request.
- Which rules apply? Scan `docs/standards/RULES.md` for the areas you touch and note the IDs.
- What is the failing test? Write it first. For a lint rule, the fixture and the message. For a block, the round-trip. For an endpoint, the OpenAPI entry and the request test.
- Which newtypes carry the data? `Slug`, `SiteId`, `Locale`, `AbsoluteUrl`, `SitePath`, `AnchorText`. A bare `String` in a signature is a smell.

## While you write

- Errors: a `thiserror` enum per library crate, cause kept with `#[source]`, no `anyhow` in libraries, no panics outside tests (BL-ERR-1 to BL-ERR-3).
- The person sees what, why, fix. Three short sentences, plain words, no em dash (BL-ERR-4, BL-TXT-1, BL-TXT-2).
- Time through `Clock`. Settings through `Config`. SQL binds. Logs through `tracing`. Secrets in `Secret<T>` (BL-RUN-1 to BL-RUN-5).
- Order that reaches output is a `Vec`, `BTreeMap` or `IndexMap` (BL-TYP-5).
- Outbound HTTP through `Outbound` only (BL-RUN-8). Paths through `SitePath` only (BL-SEC-4). Person-written HTML through `ammonia` (BL-SEC-1).
- Blocking or heavy work in `spawn_blocking` (BL-RUN-7). Nothing computed at request time that belongs at publish time (BL-PERF-2).
- Every public item documented with a one-line summary and `# Errors` where it returns `Result` (BL-DOC-1).
- A suppression is `#[expect(lint, reason = "...")]` and nothing else (BL-LNT-1).
- Functions under about 80 lines, files under about 600, six parameters at most (BL-SIZE-1).
- Content files are the source of truth. Derived data lives in `data/` (BL-FS-1, BL-FS-2).
- Every block has a directive and a round-trip test (BL-FS-5, BL-TST-5).

## Founder rules that reach code

- Light theme only in the admin. No dark mode.
- Admin screens: static HTML mockup under `docs/mockups/` and the founder's yes first.
- No placeholder text in any UI. Real example content in mockups.
- Never print, log, paste or commit a secret.
- Nothing is "working" until an end-to-end test proves it.

## Before you hand back

Run `just check`. Then load `khalid-review` and score the rubric honestly. Then write the pull request with the template. Then stop.

## Budgets you must not break

Binary under 30 MB. Cold start under 100 ms. Idle under 50 MB. Post render p99 under 5 ms. Public CSS under 20 KB compressed. Zero JavaScript to read a post. Lighthouse 100 in all four. Full numbers and how they are measured: `docs/standards/performance.md`.
