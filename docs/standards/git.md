# Git and pull requests

- `main` is protected. Everything lands by pull request with green CI and one review.
- Branch names: `feat/<phase>-<slug>`, `fix/<slug>`, `docs/<slug>`, `chore/<slug>`. Example: `feat/p1-routes-toml`.
- One task, one pull request, under 400 changed lines excluding generated code, fixtures and lockfiles. If a task needs more, split it into stacked pull requests, each useful alone.
- Conventional Commits with a scope: `feat(lint): block posts with no internal link`, `fix(render): keep heading ids stable`, `perf(render): reuse the output buffer`, `docs(adr): record the SQLite decision`, `test(content): round-trip the product card`, `chore(ci): pin cargo-nextest`.
- The commit body says why. The diff says what.
- End every commit with `Co-Authored-By: <model name> <noreply@anthropic.com>`.
- Squash merge. The squash message is the pull request title and body.
- Never force-push `main`. Never rewrite a pushed branch another agent has checked out.
- Never commit `target/`, `node_modules/`, `.env`, a database file or a secret. `.gitignore` is not optional.
- Tags are semver: `v0.1.0`. Releases are built by CI from a tag, never by hand.
- `CHANGELOG.md` is written for users in plain language, one line per change, grouped by Added, Changed, Fixed, Removed. Every pull request that changes behaviour adds its line.
