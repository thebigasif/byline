# Performance budgets

Fast is a feature. These numbers are gates, not goals. CI measures them on every pull request against `perf/baseline.json`, and a regression beyond the allowed drift fails the build (BL-PERF-1). Updating the baseline is a deliberate change in its own pull request with the reason.

## Budgets

| Measure | Budget | Drift allowed | How CI measures it |
|---|---|---|---|
| Release binary, stripped, Linux x86_64 | under 30 MB | 2% | `ls -l` after `cargo build --release` |
| Cold start to first byte | under 100 ms | 10% | `hyperfine` on `khalid serve` with a 1,000-post fixture, first request |
| Idle memory, one site, 1,000 posts | under 50 MB RSS | 10% | `/usr/bin/time -v` after warm-up |
| Post page render, in process | p50 under 1 ms, p99 under 5 ms | 10% | `divan` benchmark over the fixture |
| Cached post page, 2 vCPU | at least 10,000 requests per second | 10% | `oha` for 30 seconds |
| Full rebuild, 10,000 posts | under 30 s | 10% | timed `khalid build` |
| Incremental rebuild, one post | under 50 ms | 10% | timed file change to updated page |
| Lint one post | under 20 ms | 10% | `divan` |
| Full site audit, 1,000 posts | under 60 s | 10% | timed `khalid audit` |
| Lighthouse, default post page | 100 in all four categories | none | Lighthouse CI against the demo site |
| Largest Contentful Paint, demo site on a 5 dollar VPS class runner | under 1 s | none | Lighthouse CI |
| Cumulative Layout Shift | 0 | none | Lighthouse CI |
| Public CSS, compressed | under 20 KB | 5% | size of the built stylesheet |
| JavaScript required to read a post | 0 bytes | none | scripts-off test |
| Admin initial JavaScript, compressed | under 350 KB | 5% | Vite build report |
| Test suite, CI | under 4 min | none | job time |
| Release build, CI | under 10 min with cache | none | job time |

## How we stay inside them

- **Compute at publish, not at request** (BL-PERF-2). Rendered HTML, schema, meta, link graph and search index are produced when content changes and cached by content hash. A request copies bytes.
- **Images at upload** (BL-PERF-5). Sizes and formats are generated when the file arrives, in `spawn_blocking`.
- **Streaming and bytes.** Responses are `Bytes`. Templates render into a reused buffer. Shared immutable data is `Arc<str>` or `Arc<[u8]>`.
- **SQLite tuned.** WAL, `synchronous=NORMAL`, prepared statements, one writer, a read pool, indexes checked with `EXPLAIN QUERY PLAN` in tests for every list query.
- **Precompressed assets.** Brotli and gzip at build, served with immutable cache headers and hashed names.
- **No surprises in the hot path.** No network, no disk except cache miss, no locks across awaits, no allocation storms. The render benchmark counts allocations and fails on a 10% rise.
- **mimalloc** in the binary.
- **Profiles before guesses.** `cargo flamegraph` or `samply` before any optimisation. The pull request shows the before and after numbers.

## Fast to build

- `sccache` and `mold` in CI. `cargo nextest` with the default partitioning.
- `cargo check` on every push, the full suite on pull requests.
- `cargo bloat --release --crates` in the job summary, so a heavy dependency is seen the day it arrives.

## Baseline

`perf/baseline.json` holds the last accepted numbers and the fixture version. `scripts/perf.sh` runs every measure that can run on a CI runner and writes `perf/current.json`. The gate compares the two.
