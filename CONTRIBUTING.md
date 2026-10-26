# Contributing

Thank you for looking. Khalid is in the planning stage, and most code is written by coding agents against `CLAUDE.md`. People are welcome too, and the same rules apply to everyone.

## Before you start

1. Read `docs/plan.md` for what we are building and what is out of scope.
2. Read `docs/standards/RULES.md`. Reviews cite its IDs.
3. Look for an open issue or a task in `docs/phases/`. Comment on it before starting, so two people do not build the same thing.

## The short version

- One task, one branch, one pull request, under 400 changed lines.
- Write the failing test first.
- Run `just check` before you push. CI runs the same checks.
- Fill the pull request template, including the rubric in `docs/standards/review.md`.
- Plain language in everything a person reads, and no em dashes anywhere.

## Contributor licence agreement

Your first pull request will ask you to sign `CLA.md` through a bot. It takes one click and you sign once. It lets SerpDad keep using every contribution in its own products, which is what keeps the project funded.

## Reporting a bug

Use the bug template. Say what you did, what you expected, what happened, and the version from `khalid --version`. Never paste a key, token or password.

## Proposing a rule

Every rule in `RULES.md` needs an ID, a published source and an enforcement. Open an issue with those three things and an example of the code it would catch.
