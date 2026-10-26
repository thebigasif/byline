# 0026. Large sites are generated, and the measures live in their own crate

**Status:** Accepted, 2026-10-26. Phase 1 task 1.21.

## Context

The budgets in `docs/standards/performance.md` are measured on a site of 1,000 posts and one of 10,000. Committed as files, the larger site is about 40 MB of markdown that every clone downloads and every diff can touch. The benchmarks for render and lint also need the reader, the renderer and the linter together, and those crates belong to different tracks that change them in parallel.

## Decision

- `crates/khalid-perf` writes both sites on demand with `Fixture::THOUSAND` and `Fixture::TEN_THOUSAND`. Post `n` is the same bytes on every machine: the words come from fixed lists and a seeded generator, never from the clock or the system's random source.
- Each generated post is about 600 words in four sections, with three links to other posts, one to an outside source, an author and a category, so the reader, the renderer and the linter all do real work. Its sentences are built from four slots, so two posts rarely share a run of five words and the near-copy rule does not see one long text.
- The `divan` benchmarks for render and lint, and the measures `scripts/perf.sh` runs, live in this crate's `benches/` and `src/`, not in the crates they measure. A benchmark then never blocks a change to the crate it measures, and one crate depends on everything a measure needs.
- `scripts/perf.sh` writes the sites under `target/perf/`, never under `fixtures/` or any `content/` folder.

## Consequences

- The repository stays small and the sites cannot drift from the generator.
- `crates/khalid-perf` is never part of the `khalid` binary, so it adds nothing to the binary size budget.
- Changing the generator changes what every measure means. Such a change is a baseline change: its own pull request, with the before and after numbers.
