# How Khalid is built

This page explains how work moves from a task to `main`, so anyone can take part: a person, or a coding agent you run yourself. It describes the process and the tools. It does not describe or grant access to any machine; the maintainers' build server is private, and nothing here needs it.

## Who does what

- **The founder** decides product and brand questions. They have said yes in advance to everything in the plan, so work never waits on them.
- **The maintainer** (today a reviewing Claude session) writes task briefs, answers questions in `docs/decisions/`, reviews every pull request and lands it on `main`. Only the maintainer merges.
- **Builders** write the code. Most are Claude Code sessions, one per track, each in its own git worktree and its own crates. People are builders too, under the same rules.

## Where the work is written down

| What | Where |
|---|---|
| What must be true | `docs/PRD.md` (the `KH-*` requirements) |
| How it is built, phase by phase | `docs/plan.md` |
| The live task list with statuses | `docs/phases/` |
| Answers to questions | `docs/decisions/` |
| Architecture decisions | `docs/adr/` |
| Rules every review cites | `docs/standards/RULES.md` |
| What is missing, requirement by requirement | `docs/audits/` |
| The contract for coding agents | `CLAUDE.md` |
| The standing brief every builder gets | `scripts/maintainer/builder-brief.md` |

## From task to pull request

1. Pick a task in `docs/phases/` or a brief in `docs/audits/`. Say so in an issue first, so two people do not build the same thing.
2. Branch `task/<slug>` from a fresh `main`. A task can have several parts; each part is its own pull request under 400 changed lines, and later parts may stack on earlier ones of the same task. Never stack on someone else's branch.
3. Write the failing test first, then the code. Run `just check` before every push.
4. Open the pull request with the template, the rubric from `docs/standards/review.md`, and the `KH-*` IDs it satisfies.
5. Leave the shared files alone: status lines in `docs/phases/`, `docs/adr/README.md` and `CHANGELOG.md`. Put your changelog line in `changelog.d/<branch-slug>.md`, name a new ADR `0000-<slug>.md`, and suggest a status line in the pull request body. The maintainer fills these in when it lands.
6. If a choice is missing, do not stop. Take the option that best fits the plan and the decisions, and write it under "Decisions taken" in the pull request.

The three required checks are Text rules, Rust and Admin. Performance budgets also run and must hold; a budget is never loosened to make a change pass.

## How work lands: the stack train

`scripts/maintainer/train.sh` lands ready work in batches.

1. It reads every open pull request and builds chains: a pull request on `main`, then each single child stacked on it.
2. It retargets the top of each chain to `main`. The top's CI ran on the whole stack, so one green run covers every part.
3. If the top no longer merges, `resolve.sh` merges `main` into it, but only for mechanical conflicts: `Cargo.lock`, dependency lists, the lint catalogue and docs. A conflict in code is never settled by taking one side's whole file, because that silently drops work; it goes back to a builder, who merges by hand and keeps both sides.
4. It waits for the required checks, and pushes an empty commit to rerun a check stuck for over 15 minutes.
5. It prints one review line per pull request: tests added, crates touched, and any lint suppression outside tests. A stack with a suppression waits until the maintainer has read it and approved it.
6. It squash-merges the top with a message listing every part, then closes each lower pull request only after checking that every real commit of it is inside what landed.

When several stacks keep conflicting in the same central files, a builder makes one integration branch that merges them all by hand. The train then lands that integration alone, first, before anything else can move `main` (`ONLY=<number>`).

## Running builders

`scripts/maintainer/run-track.sh <name> <worktree> <brief>` starts one headless Claude Code builder with `builder-brief.md` plus a task brief. The builder gets an allowlist of commands and cannot merge, force-push, use sudo, or run npm packages it fetched by name; the maintainer generates lock files. One task per run keeps every task on a fresh context. Each run clears its build output when it passes 15 GB.

You can run builders on your own machine with your own Claude Code account and your own fork. Open the pull requests against this repository and they go through the same train.

## Safety

- No server address, login, key or token is ever written in this repository, in a brief or in a pull request. Secrets live only in the environment of the machine that needs them.
- Builders never see the maintainers' credentials beyond what their own tools need to open pull requests.
- Tools run only from pinned, locked dependencies. `scripts/check.sh` never lets `npx` fetch a package by name.
