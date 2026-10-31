# 0000. Git mode: a branch per draft, and a signed push hook that pulls

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase 1, gaps brief G53 (KH-EDT-4), plan sections 4.6 and 5. Taken under the founder's yes in advance of 2026-10-27.

## Context

Plan section 4.6 says git mode is optional: each draft is a branch named after the post and a pull request, publishing is the merge, and the blog pulls on push through a webhook or `khalid pull`. `khalid pull` exists (ADR "`khalid pull` only fast-forwards"). The plan does not say how git mode is switched on, what the branch is called, who opens the pull request, or how a push webhook is trusted. Drafts made through the API live in `drafts/` (ADR 0030); a developer in git mode writes in their own editor, where `khalid new` writes straight into `content/`.

## Decision

- Git mode is a way of working, not a site setting: `khalid new "Title" --branch` starts the branch `post/<slug>` from where the repository stands, writes the post there, and commits only that file as "Draft: Title". The branch is the draft, so the post can sit in `content/`; `main` never sees it until the merge. No change to `site.toml` or `khalid-core` is needed.
- Khalid never pushes and never opens the pull request. It prints the `git push` line and says that merging publishes. Hosts differ in how they open a pull request, and pushing needs the writer's own credentials.
- A branch that already exists, or a post file that already exists, is refused before anything changes. Outside a repository the message says to run `git init`.
- The push hook is `POST /api/hooks/git` on `khalid serve`. It answers only when `KHALID_GIT_HOOK_SECRET` is set in the environment, and only to a request signed with it: GitHub's `X-Hub-Signature-256` (HMAC-SHA256 of the body) or GitLab's `X-Gitlab-Token`, both compared in constant time. It runs the same fast-forward pull as `khalid pull` in the background and answers 202 at once, since hosts give a hook about ten seconds. Pulls never overlap. The server's file watch then shows the merged post.

## Consequences

- A static site still needs `khalid pull` from a host's deploy step, since nothing listens there.
- Every git step runs the `git` program with the writer's settings, as `khalid pull` does; git must be installed for git mode.
- Comments on drafts made through the admin stay keyed by the post's identity, so they follow a draft whichever branch it is on.
