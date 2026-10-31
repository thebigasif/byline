# 0000. `khalid pull` only fast-forwards, through the git command

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase 1, gaps brief G32 (KH-OPS-2), plan sections 4.1 and 5.14. Taken under the founder's yes in advance of 2026-10-27.

## Context

The plan says `khalid pull` fetches the content repository in git mode and rebuilds. It does not say how the fetch is done, what happens when the blog's own copy has changes, or what "rebuild" means for each mode. A running `khalid serve` already watches the site folder and shows changed files on its own.

## Decision

- `khalid pull` runs the `git` program on the computer, `git pull --ff-only`, with prompts for passwords turned off. It uses the user's own git setup: remotes, keys and credential helpers. No git library is linked into the binary.
- It only fast-forwards. When the blog's copy has commits or edits the repository does not, it stops with what, why and fix, and changes nothing. Publishing is the merge (plan 5.14), and that merge happens in the repository, not on the blog's server.
- After a pull, the site is checked the way every command checks it at start: settings, routes and content. A broken push is reported, with exit 1.
- "Rebuild" depends on the mode. A static site is built again into `public/`, through the same publish gate as `khalid build`, when anything changed. A server or headless site needs nothing more: `khalid serve` sees the new files through its watch.

## Consequences

- Git must be installed for git mode. Without it, `khalid pull` says so and how to install it.
- No password prompt can hang a script or a webhook: a remote that needs one fails with git's own first line in the message.
- The push webhook the plan mentions can call the same function later; it is not built here.
