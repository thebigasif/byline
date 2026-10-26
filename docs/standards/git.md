# Git

Trunk-based, because the founder wants speed. Every finished task is pushed straight to `main`.

- `main` is the only long-lived branch. It refuses force pushes and deletions, history stays linear, and the three CI checks must be green on a commit before it can land there.
- One task, one landing. Push your commits to `task/<slug>`, wait for CI with `gh run watch`, then `git push origin task/<slug>:main` and delete the remote branch. A few commits per task are fine. Each commit is small enough to read in one sitting.
- Before every push: `just check` green, the self-review rubric from `docs/standards/review.md` scored, the phase brief updated.
- `main` never sees red, because only green commits are fast-forwarded into it. A red check on your task branch is fixed on that branch before anything else.
- Short-lived local branches are fine for your own work. Fast-forward them into `main`; never leave them on the remote.
- Pull requests only for the exceptions in `CLAUDE.md`: a rule, CI or budget change, or a diff the founder needs to see before deciding.
- Conventional Commits with a scope: `feat(lint): block posts with no internal link`, `fix(render): keep heading ids stable`, `perf(render): reuse the output buffer`, `docs(adr): record the SQLite decision`, `test(content): round-trip the product card`, `chore(ci): pin cargo-nextest`.
- The commit body says why, how it was tested, the rubric score, and what the founder can look at. The diff says what.
- End every commit with `Co-Authored-By: <model name> <noreply@anthropic.com>`.
- Never force-push `main`. Never rewrite history another session may have fetched.
- Never commit `target/`, `node_modules/`, `.env`, a database file or a secret. `.gitignore` is not optional.
- Tags are semver: `v0.1.0`. Releases are built by CI from a tag, never by hand. Prerelease tags such as `v0.1.0-alpha.1` are for tests and get deleted after.
- `CHANGELOG.md` is written for users in plain language, one line per change, grouped by Added, Changed, Fixed, Removed. Every commit that changes behaviour adds its line.

Where any other standard in this repository says "pull request", read "commit" in this trunk-based mode.
