# The demo check is one script with pinned Node tools

Number: next free

**Status:** Accepted, 2026-10-26. Phase 1 task 1.22.

## Context

Task 1.22 asks for the W3C HTML validator, Lighthouse and axe on every page type of the demo site, and again with scripts off at six widths. `docs/standards/testing.md` places end-to-end journeys in a Playwright suite under `e2e/`, which does not exist yet. The tools are Node and Java programs that never ship in the binary, and headless browsers may not run on the founder's 8 GB laptop.

## Decision

- `scripts/demo-check.sh` does the whole check: it builds the binary, builds and serves the site, and runs each tool. `just demo-check` calls it, and CI runs the same script.
- The tools are pinned to exact versions in `scripts/demo-check/package.json`: Lighthouse, Playwright, `@axe-core/playwright` and `vnu-jar`. They are test tools only. axe-core is MPL-2.0, which `docs/adr/0005-permissive-dependencies.md` limits for linked code, and nothing links it.
- The site is served with `KHALID_ADDRESS` set to `http://localhost:<port>`, so canonical links point at the domain Lighthouse checks, and localhost counts as a secure origin without HTTPS.
- Pages come from the site's own sitemaps, so every listed page is checked, not a sample, and a new page type is covered without a change here. The not-found page in each language is added, held to the performance and accessibility categories, because it answers 404 on purpose.
- The widths are 320, 375, 768, 1024, 1280 and 1920 px, the range in RULES.md BL-WEB-3. `FRONTEND.md` lists 414 and 1440 instead of 1280 and 1920; the task brief's list is used.
- Every check runs even when an earlier one fails, so one run lists every problem.

## Consequences

- The journeys in `e2e/` can reuse `scripts/demo-check/` when they are written, or replace `browse.mjs`.
- There is no lockfile yet, so tools below the pinned four can change between runs. A lockfile comes with the first run that may use `npm`.
- Lighthouse runs once per page at its default mobile settings. Desktop scores are not gated.
