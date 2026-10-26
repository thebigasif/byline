# Khalid, by SerpDad: how agents work in this repository

Khalid is the product's name, chosen by the founder on 2026-10-26. Older files may still say Byline or Wellranked; those are the earlier working names and mean the same thing. The product is an open-source blog engine in Rust: one binary, text files as the source of truth, SEO defaults built in, a linter that refuses to publish a weak post, and a public API. The founder does not write code. Claude Opus 5.5 writes it, and other agents review it. This file is the contract between them.

## Read these first, in this order

1. `docs/plan.md`: what we build and why. Section 1 holds the founder's decisions. Do not reopen them.
2. `docs/standards/RULES.md`: every rule with an ID. Reviews cite the IDs.
3. `docs/phases/`: the current phase brief. Work only on tasks listed there.
4. `docs/adr/`: architecture decisions. Add one when you make a choice the plan does not cover.
5. `docs/standards/`: `rust.md`, `api.md`, `typescript.md`, `performance.md`, `testing.md`, `review.md`, `git.md`.
6. `FRONTEND.md`: the front-end checklist every template follows.

The skills in `.claude/skills/` load on their own. `rust-best-practices` and `rust-review` come from BeautifulRust (MIT, Morgan Linton). `byline-way` and `byline-review` add this repository's rules on top. When they disagree, `byline-way` wins, because it carries the founder's decisions.

## The founder's rules. Never break these.

- **Light theme only.** No dark backgrounds, no dark mode toggle, anywhere in the admin. The public blog follows the main site's colours through theme sync.
- **Plain language in every message a user sees.** What happened, why it matters, what to do. One short sentence each. Define a term the first time it appears.
- **No em dashes anywhere.** Not in code comments, docs, UI text, commit messages or pull requests. Use a full stop, a comma or a colon.
- **Don't make me think.** One focus point per screen. No visible placeholders. Every decision screen explains its context and the consequence before it asks.
- **Admin screens need a mockup and the founder's yes before they are built.** Ship the mockup as a static HTML file under `docs/mockups/` and stop.
- **Never handle secrets.** Never print, log, paste or commit a key, token or password. Tests use generated values. The founder enters real secrets through hidden input.
- **Prove it before you say it works.** A flow is done when an end-to-end test exercises it, not when the code compiles.

## How a task flows

The founder wants speed. There are no pull-request round trips. Each finished task goes straight to `main`.

1. **Pick one task** from the current phase brief. Work on `main` in the repository folder, or on a short-lived local branch you fast-forward into `main` yourself.
2. **Write the failing test first.** A lint rule starts with its fixture and its message. A block starts with its round-trip test. An endpoint starts with its OpenAPI entry and a request test.
3. **Write the code** with `rust-best-practices` and `byline-way` loaded. Check `RULES.md` IDs as you go. Make invalid states unrepresentable before you write the first `if`.
4. **Run `just check`** (or `scripts/check.sh`). Format, clippy with warnings as errors, tests, docs, dependency audit, unused dependencies, typos. Everything green locally before you push. Never push red.
5. **Self-review** with `rust-review` and the rubric in `docs/standards/review.md`. Fix everything it finds. Put the rubric score and anything below 10 in the commit body.
6. **Commit and push.** A Conventional Commit whose body says why, how it was tested, the rubric score, and what the founder can look at. Push it to a short-lived branch named `task/<slug>`.
7. **CI is the gate, and you hold the key.** Wait for the three required checks on that commit with `gh run watch`. When they are green, fast-forward `main` yourself: `git push origin task/<slug>:main`, then delete the remote branch. No pull request, no reviewer, no round trip. If a check is red, fix forward on the same branch and push again; `main` never sees red.
8. **Pull requests are for the exceptions:** a rule change, a change to CI or budgets, a founder question that needs a diff to discuss. Everything else lands by fast-forward.

## Code rules in one screen

Details and IDs are in `docs/standards/RULES.md`.

- No `unsafe`. It is forbidden at the workspace level.
- No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, slice indexing or `as` casts outside tests. Return errors.
- Each library crate has its own error enum built with `thiserror`. The binary turns errors into a user message with what, why and fix. `anyhow` never appears in a library crate.
- Every suppression is `#[expect(lint, reason = "...")]` on the narrowest item. Never `#[allow]`.
- Every public item has docs with a one-line summary, and `# Errors` or `# Panics` sections where they apply.
- Time comes from a `Clock`. Settings come from one typed config read once at start. SQL always binds its values. Output goes through `tracing`.
- Where order reaches a user, a page or a file, use `Vec`, `BTreeMap` or `IndexMap`. Never `HashMap`.
- Functions stay under about 80 lines and six parameters. Files stay under about 600 lines. A crate does one thing.
- Content files are the source of truth. Nothing derived is ever written into `content/`.
- The API is the only door. The CLI, the MCP server, the admin and the SerpDad publish target call the same functions the API calls.
- Every editor block has a markdown directive and a round-trip test.
- Every outbound HTTP request goes through the one shared client, with timeouts, size limits and the private-network guard.
- A new dependency needs one sentence in the commit body saying why, a permissive licence, and a green `cargo deny`.

## Performance is a feature with a budget

The budgets are in `docs/standards/performance.md`. CI measures them against a committed baseline. A change that breaks a budget does not merge, however good it looks. Measure before you optimise, never guess, and never regress.

## Front end

`FRONTEND.md` is the checklist. Semantic HTML with one `h1`. Every public page works with scripts off. Responsive from 320 px. WCAG 2.2 AA. Zero layout shift. No third-party scripts by default. The admin is the one part that may need JavaScript. It is TypeScript in strict mode with Biome, see `docs/standards/typescript.md`.

## Definition of done

A task is done when all of these hold:

- Tests exist for the behaviour and its failure modes, and they pass.
- `just check` is green, and so is CI.
- Docs are updated: rustdoc, `docs/`, the OpenAPI spec, the CLI help.
- The done line in the phase brief is met, word for word.
- The commit body shows the rubric at 10/10, or says which point is below and why.
- No founder rule is broken and no budget has regressed.
- A short demo note in the commit body tells the founder what to look at and how, in plain words.

## When you are unsure

Product decisions belong to the founder. They are listed in `docs/plan.md`, sections 1 and 12. Do not decide them yourself. Write the question under "Questions for the founder" in the phase brief, mark the task blocked, push, and move to the next task. The CTO session answers in `docs/decisions/`.

Technical decisions the plan does not cover are yours. Make the call, record it in `docs/adr/`, and say so in the commit body.

## Commits and attribution

Conventional Commits: `feat(lint): block posts with no author`. Subject under 72 characters, body says why. End every commit with `Co-Authored-By: <model name> <noreply@anthropic.com>`. Never force-push `main`. Never commit secrets, `target/`, `node_modules/` or `.env` files.
