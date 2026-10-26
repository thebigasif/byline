# `khalid build` writes nothing when the publish gate refuses a post

**Number:** next free.

**Status:** Accepted, 2026-10-26. Phase 1 task 2.6.

## Context

Task 2.6 makes `khalid build` refuse to publish a post the publish gate blocks, with exit code 1 and every finding in plain words. The plan says the linter "refuses to publish a post that breaks basic SEO hygiene", but not what happens to the rest of the site. A build could leave the refused post out and write the others, or write nothing. The Admin API's publish path will ask the same gate one post at a time, and it has not landed yet.

## Decision

- **A refused post stops the whole build, before any page is written.** The output folder keeps the last good build. Leaving one post out would also change the home page, the author pages and every link to that post, and a site that quietly lacks a post looks published when it is not.
- **Only dated posts are checked.** A post with no date is a draft, which `khalid build` leaves out, so the gate leaves it alone too. Removing `date:` is the documented way to keep a post back.
- **The output is `khalid lint`'s, for the refused posts only:** every finding, warnings included, in the same sentences, then one message with what, why and fix. With `--json` it is `{"error": {...}, "refused": [report, ...]}`, each report the shape `khalid lint --json` prints.
- **Build asks the quick question first.** `PublishGate::refuses` runs only the rules set to block and stops at the first hit; only a refused post gets the full `verdict` with its warnings. Several warnings compare a post with every other post, which is right for one post at publish and quadratic for a whole site: run on every post, the full verdict took over nine minutes on the generated 10,000-post site, against a 30-second budget. The quick question gives the same answer, because a warning can never refuse a post, and a test holds the two together.
- **The seam is `khalid_lint::PublishGate`,** a trait with `verdict` and a provided `refuses`, that `Linter` implements. `khalid build` takes a `&dyn PublishGate`, and the Admin API's publish path takes the same, so both ask one question and tests can stand in for the rules.

## Consequences

- One blocked post holds back the whole site until it is fixed or turned back into a draft. That is strict on purpose: the founder's rule is that a weak post is refused, and the message names the way out.
- `khalid new` writes a dated post with no author and no internal link, so a build right after it is refused until the writer adds both. Whether `new` should write a draft instead is a question for the founder in the phase brief.
- Site rules, such as orphan post, never run at build time, as at publish (2.5): a new post has no inbound links yet.
- On the generated 10,000-post site the gate adds about 0.3 seconds of processor time to a full build (1.6 to 1.9 seconds of user time, release build, this server). A `lint.toml` that sets a site-wide rule such as `near-duplicate` to block makes the build pay that rule's cost on every post. Making those rules grow less with the site is task 1.21's lint measure.
