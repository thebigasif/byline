# Squash reverts audit, 2026-10-27

What this is: a check of every squash commit on `main` since 2026-10-26 for work that main already had and that a squash then removed by accident. The cause: before landing a stack, a script sometimes merged main into the stack's top and, on a conflict in a code file, kept the branch's whole file. The squash then carried that file onto main, which reverted whatever main had gained in it.

Checked at `c4406d9` (#303), the head of `main` when the audit ran.

## How it was checked

1. Every squash on `main` since 2026-10-25 whose subject ends in `(#NNN)`, from #1 to #303. Each pull request head was fetched from `refs/pull/NNN/head`.
2. **Merges inside each stack.** For every merge commit in a pull request's history, the merge was redone with `git merge-tree` and compared with what was recorded. A file that matched the branch's side exactly while main's side had changed it was flagged. This found many flags, almost all from tracks resolving conflicts against an earlier squash of their own work, which is expected.
3. **Removed lines with no author in the stack.** For each squash, every line it removed (`git diff <squash>^ <squash>`) was compared with every line the stack's own non-merge commits removed. A line that no commit of the stack removed came out through a merge or the squash itself. Only lines still missing from `main` today were kept. This is the test that decided each case below.
4. **Lines put back.** The same comparison for added lines found records and references that came back under an old ADR number after a renumbering.
5. Each remaining case was read by hand against the commit that added the line and the ADRs, then marked intended or accidental.

Limits: a removed line that the stack also removed somewhere else in the same file, word for word, is treated as intended. A one-line change in a doc comment has no test that would catch it.

## The suspects named in the brief

| Suspect | Result |
|---|---|
| `crates/khalid/src/main.rs`: `lint`, `lint --fix`, `new`, `build` wiring | Intact on `main`. `db`, `lint` (with `fix`), `new`, `build`, `keys` and `serve` are each dispatched, and each needs `--site`. The old `Lint { path }` arm that #212 and #230 removed was replaced on purpose by `Lint { path, fix }`. |
| `crates/khalid/src/*.rs` | #254 lost main's SEO files in `build.rs` and `build_files.rs` during a merge. #256 restored them, so nothing is missing today. #256's own edits to `build.rs` are intended (the stylesheet wiring). |
| `lib.rs` and `main.rs` module lists | No module declaration is missing on `main`. |
| `Cargo.toml` dependencies | None lost. The only `Cargo.lock` hits are `quick-xml` and `roxmltree` gaining version suffixes when a second version arrived (#167). |

## Accidental reverts found and restored

| Squash | What was lost | Added by | Restored in |
|---|---|---|---|
| #222 | Theme sync skipping print, dark-scheme and alternate stylesheets: `SkipReason::NotForScreen`, `light_screen`, and the test `print_dark_and_alternate_stylesheets_are_skipped_and_screen_ones_are_read` | #170 | #313 |
| #222 | The ADR reference in `crates/khalid-theme/src/links.rs` went back from 0015 to 0012 | #170 | #313 |
| #222 | A stale copy of ADR 0015, missing its last two decisions, came back as `docs/adr/0012-theme-sync-reads-the-site-it-is-given.md` | renumbered before #170 | #313 (removed) |
| #222 | The crate docs and Cargo description said the theme crate prints proxy rules, which ADR 0017 had moved to `khalid-mount` | stale stack text | #313 |
| #65 | The renumbered ADR came back under its old number as `docs/adr/0009-seo-outputs-from-their-own-inputs.md` | d6f2b30 renamed it to 0013 | this pull request (removed) |
| #65 | `crates/khalid-seo/src/lib.rs` cited ADR 0009 again instead of 0013 | d6f2b30 | this pull request |
| #56, #61 | `crates/khalid-webhooks/src/verifier.rs` cited ADR 0010 again instead of 0014 | d6f2b30 | this pull request |

The test that would have caught them:
- `crates/khalid-theme/tests/links.rs` covers the stylesheet filter.
- `crates/khalid/tests/adr_records.rs` refuses two ADR files with the same title under different numbers. On `main` it fails for both stale copies.

## Removals that were intended

- Phase 0 (#3, #4, #6, #7, #17): the Byline to Khalid rename, the move of `lints/check-reference` to `lints/check`, and lock file churn.
- Status lines in `docs/phases/*.md` and rows in `docs/adr/README.md` and `CHANGELOG.md` that later merges rewrote. These belong to the CTO, and #231 rewrote every status line on purpose.
- Every other flag from step 2: a stack replaced main's version of its own earlier part with a later one.

## Left for the CTO

These are not reverts, so this audit did not change them. They need the CTO, who owns the files or the numbers.

- `docs/adr/README.md` lists "0010 Webhook event names and signatures" next to the correct "0014" row. It lists 0011 "The lint engine..." twice. It has no rows for 0012 theme or 0013 theme tokens, and its rows are out of order.
- Two numbers each hold two different decisions: 0011 (API keys, and the lint engine) and 0013 (SEO outputs, and theme tokens from #222). Per the numbering rule, the later merger renumbers. With the stale copies removed, 0009 and 0012 no longer collide.

## Second run, after #303 to #564

Asked for by brief G1 of `docs/audits/2026-10-27-gaps-2.md`, because the loss of plan section 4.12 in #287 slipped past the first run.

Checked at `2446341` (#564): all 43 squashes that landed after `c4406d9`. That includes #266, #275, #287 and #291, whose numbers are below 304 but which landed after the first run. Every pull request head came from `refs/pull/NNN/head`.

**Why step 3 missed #287.** The loss was not made by a merge or the squash. A normal commit in the stack, `b55cdba`, saved an old copy of `docs/plan.md` and `docs/phases/phase-1.md`, so step 3 counted the removal as the stack's own work. A second pass was added for this: every line a stack commit removed that another squash had added to main, that the stack never added itself, and that is still missing on main.

| Squash | What was lost | Added by | Restored in |
|---|---|---|---|
| #287 | `docs/plan.md` section 4.12, Appendix A.3, and "the reader features from 4.12," in the Phase 1 summary | #258 | this pull request |
| #287 | `docs/phases/phase-1.md` task 1.23 Reader features, and "then 1.23 on top of them" in the Order paragraph | #258 | this pull request |

No other accidental revert was found. Step 3 flagged 143 lines in nine squashes, and the second pass listed 541 groups; every one read was intended: the same edit made against an older copy of a file, "unknown field" messages and snapshots that grew on purpose, code replaced by a later part, the ADR renumbering in #319 and #314, `focus-topic-missing` split into six rules (#367, #372), commands leaving the "not built yet" list as they landed, and refactors and moves.

Limits of this run: code groups under eight lines in the second pass were judged from their file and commit message, not line by line. The second pass only traces lines a first-parent commit on main added. Steps 2 and 4 were not run again. The reader features themselves were built in #355, so only the plan and brief text was lost, not code.
