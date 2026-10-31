# 0000. A blocked post is left out of the site, by build and serve alike

**Number:** next free. **Status:** Proposed, 2026-10-31. Supersedes 0033. Fixes finding 5 of the 1.0 readiness check.

## Context

ADR 0033 made `khalid build` write nothing when the publish gate refuses one post. The 1.0 readiness check found two problems with that:

- After a WordPress import, one old post with no internal link kept every page of a static site from being written. A real import holds many such posts, so a static site stays empty until every old post is fixed.
- `khalid serve` never asked the gate. The same blocked post was live on a running site, so build and serve disagreed on what is published, and KH-LNT-1 says a blocked post "cannot be published".

## Decision

- **A post the gate refuses is left out, and the rest of the site goes live.** `khalid build`, `khalid export` and every full render of `khalid serve` call one function, `build_gate::screen`, and render the content without the refused posts. The home page, lists, feeds, sitemaps, search and the Content API never name a left-out post.
- **The build says what it left out.** It prints every finding of each left-out post, as `khalid lint` does, then "Left out 1 post, because it is not ready to publish", with the fix. `--json` adds `left_out`, each item the shape `khalid lint --json` prints. The exit code is 0, because the build did what was asked: the blocked post is still not published. A build that cannot run at all, such as one with a mistake in `lint.toml`, still exits 1 and writes nothing.
- **Serve logs each left-out post** at warn level on every full render, naming its file and pointing at `khalid lint`.
- The quick question first, then the full verdict for a refused post only, stays as 0033 set it.

## Consequences

- One old post no longer empties a site, and a running site never shows a post the build would refuse.
- A post that links only to a left-out post still goes live with that link, which then answers 404. The audit finds it. Judging the site again after leaving posts out could cascade, so it is not done.
- A save that turns a live post into a blocked one shows the saved page alone for a moment, until the full render that follows every save drops it. The first home pages at start are made before the gate runs, so they may list a blocked post for the second or two before the full render replaces them.
- Every full render of `khalid serve` now pays the gate: about 0.3 seconds of processor time on the generated 10,000-post site, as 0033 measured for build.
