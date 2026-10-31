# Performance budgets

Fast is a feature. These numbers are gates, not goals. CI measures them on every pull request against `perf/baseline.json`, and a regression beyond the allowed drift fails the build (BL-PERF-1). Updating the baseline is a deliberate change in its own pull request with the reason.

## Budgets

| Measure | Budget | Drift allowed | How CI measures it |
|---|---|---|---|
| Release binary, stripped, Linux x86_64 | under 30 MB | 2% | `ls -l` after `cargo build --release` |
| Cold start to first byte | under 100 ms | 10% | `hyperfine` on `khalid serve` with a 1,000-post fixture, first request |
| Idle memory, one site, 1,000 posts | under 50 MB RSS | 10% | resident set size of `khalid serve`, read after it settles, then five readings one second apart of which CI takes the lowest (CTO answers 2026-10-27, section 2) |
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
- **mimalloc** in the binary, with its default settings. `khalid serve` lets it hand freed memory back once its work is done. See "Allocator settings" below.
- **Profiles before guesses.** `cargo flamegraph` or `samply` before any optimisation. The pull request shows the before and after numbers.

## How the idle memory is read

The 55 MB budget and the meaning in the CTO answers of 2026-10-27, section 2, stand. What changed on 2026-10-28 is only how the reading is taken, because one binary measured 51.43 to 81.49 MB across eight runs: mimalloc returns a freed page one second after it is freed, and only when a thread later frees a page or ends, so whether tokio's idle blocking threads had ended before the reading at 15 s decided the number.

`khalid-perf serve` now reads the server's thread count and resident memory each second after the first answer, with no traffic in between. The server has settled when the trace is at least 15 s long (the wait still outlasts tokio's blocking threads, which end 10 s after their last task) and the last 3 readings show the thread count unchanged and the resident reading not falling by more than half a megabyte. A platform whose `/proc` does not say the thread count settles on its memory alone. Nothing waits for ever: after 60 s the readings are taken anyway, so a wedged server cannot hang the measure.

Once settled, five readings are taken one second apart and the lowest is the measure, so a page the allocator hands back between two readings cannot raise it. The budget number was never changed.

## Allocator settings

One setting is on: `MIMALLOC_PURGE_DELAY=0`, set in the deployment image and in the server `khalid-perf serve` measures. mimalloc holds a page it frees in its arena for a later reuse, with a reset delay before giving it back; an idle server never touches its arenas again, which is why a heap of 16.5 to 26.8 MB live read 54 to 67 MB resident. With the setting, a freed page returns to the OS at once. A delay of a hundred milliseconds was measured and rejected: the cold start did not recover (120.08 ms) and the idle reading climbed back over the budget (60.48 MB), because the pages a delayed purge would return were never touched again by an idle server. `MIMALLOC_ABANDONED_PAGE_PURGE` was measured and rejected: it made both budgets worse (idle 58.44, cold start 117.23). The measurement that justifies the setting that is on is the eight CI runs in the pull request that added it.

The setting has a cost: every page freed is handed back at once and faulted in again when it is reused, so work that allocates and frees a lot costs more. Before it answers, `khalid serve` therefore does only what the first answer needs. Measured on 2026-10-31 with timing marks in a release build on the 1,000-post site, on a shared server, the start before the first answer was: walking the content folder 12 to 18 ms, reading the posts 22 to 34 ms, checking their bodies 8 to 12 ms, the runtime and database 5 ms, the home pages 6 to 17 ms. Three steps were moved out of that path:

- **The stylesheet's compressed copies** (11 ms). Brotli at its best quality with a 4 MB window sets up tens of megabytes of tables, and the server threw both copies away: it compresses its pages after it answers. It now builds the stylesheet's text and hashed name only.
- **The Content API and Admin API router** (8 to 9 ms, most of it the OpenAPI document). It is built on a blocking thread once the routes are wired, and a request under `/api/` waits for it.
- **The full render**, which used to start before the routers were built and competed with them. It starts once the routes are ready.

The first request is answered within a fraction of a millisecond of the routes being ready. What is left is reading the content, which the first page needs.

## Allocator settings history

The CTO answers of 2026-10-27, section 2, allow tuning the allocator to hand memory back, as long as the setting is recorded here with the measurement behind it. Measured on 2026-10-27 with `khalid-perf serve`, release build, on the generated 1,000-post site, idle memory as defined in the table above. The server was shared and busy (load average 65 to 85 on 16 cores), so each setting ran twice where it mattered.

| Allocator | Idle memory, 1,000 posts | Idle memory, 10 posts |
|---|---|---|
| mimalloc 0.1.52, default settings (v3) | 60.3 MB, 63.6 MB | 35.0 MB |
| mimalloc with `no_thp` | 62.0 MB, 60.7 MB | |
| mimalloc with `v2` | 112.3 MB, 115.8 MB | |
| The system allocator (glibc), for reference | 64.5 MB | 23.4 MB |

What this shows:

- **No setting is changed.** `no_thp` makes no difference, because this kernel's huge pages are `madvise` only and mimalloc does not ask for them. `v2` holds almost twice as much. mimalloc's purge delay and decommit options can only be set through environment variables or `unsafe` calls, which the workspace forbids, so they were not measured.
- **The allocator is not what holds the memory on a 1,000-post site.** The system allocator holds as much as mimalloc there. The rendered pages are 6.9 MB plain, with their Brotli copies on top. The server's fixed cost is about 35 MB, measured on a 10-post site. The rest grows with the posts and has not been broken down yet; the Content API's store and its index refresh, the subject of the CTO answers' section 3, are the largest part not yet measured.
- **mimalloc costs about 12 MB more than the system allocator on a small site**, from its per-thread heaps. That is worth measuring again once the 1,000-post site is inside its budget.

Measured again on 2026-10-27 (perf track, task 1.21), with the server answering before its full render and each page compressed with a window of at least 128 KB, on a quiet server (load average 3 to 9), two runs each, idle memory as defined above:

| Setting | Idle memory, 1,000 posts |
|---|---|
| mimalloc, default settings | 57.85 MB, 59.47 MB |
| with `no_thp` | 59.06 MB, 60.53 MB |
| tokio blocking threads kept 1 s instead of 10 s | 58.22 MB, 59.80 MB |
| the full render on a thread of its own, which ends after it | 58.08 MB, 59.48 MB |
| two tokio worker threads, on top of one copy per page | 63.98 MB, 70.94 MB, against 65.82 MB |

None of them changes idle memory beyond the noise between runs, so still no setting is changed. A `dhat` heap profile of the running server, 14 s after start, holds 26.8 MB live, about half of it each page's plain bytes beside its Brotli copy; `khalid-server` now keeps one copy per page. Brotli's own buffers are the other large cost: below a 128 KB window the `brotli` crate set up a 32 MB table for each page.

### Handing freed memory back (task 1.21, 2026-10-27)

Why idle memory moved by 30 MB between runs of one binary: mimalloc returns a freed page to the system one second after it is freed, and only when a thread later frees a page or ends. A server with no visitors does neither. A trace of resident memory every half second showed it at 70 to 80 MB from the end of the start's render until tokio's idle blocking threads ended, 10 s after their last task, and then falling to 51 to 62 MB. Whether that fall came before the reading at 15 s decided the number. A `dhat` profile at 14 s held 16.5 MB of live data.

`khalid serve` now does two things (ADR "khalid serve hands freed memory back once its work is done"):

- Its blocking threads end 1 s after their last task instead of 10 s.
- 2 s after its pages are replaced or the Content API's feed finishes a catalog, with nothing else in between, it ends one short-lived thread, which makes mimalloc return everything freed by then.

Measured with `khalid-perf serve`, release build, generated 1,000-post site, the three settings run in turn three times on a busy server (load average about 65 on 16 cores):

| Setting | Idle memory, 1,000 posts |
|---|---|
| The thread ended after work, alone | 64.35 MB, 58.61 MB, 63.91 MB |
| With blocking threads kept 1 s | 56.07 MB, 52.48 MB, 51.36 MB |
| With that, and the content read on a thread that ends | 56.34 MB, 52.00 MB, 59.45 MB |

Main measured 51.43 to 81.49 MB in eight runs. Reading the content on its own thread made no difference and was dropped. Neither did mimalloc's `no_thp` here (55.0, 57.6 and 57.2 MB, against 49.8, 53.2 and 68.5 MB without it), although one run without it had 12 MB in huge pages at idle.

CI's runner tells another story. With the hand-back, its trace (run 40470523235) fell to 60.22 MB six seconds after the first answer, then rose to 71.82 MB at eleven seconds and stayed there, with 24.00 MB of the 51.07 MB of anonymous memory in 2 MB huge pages. On the 10,000-post site, 172 MB of 270.50 MB was. mimalloc asks Linux for huge pages over its whole arena, and the kernel's background collapser, which looks every ten seconds, then fills whole 2 MB pages around the little memory still in use there. So the binary now builds mimalloc with `no_thp`, which stops it asking. That alone took CI to 63.52 MB with 14.00 MB still in huge pages (run 40471366129), because CI's runners are set to `always`: Linux hands out huge pages there without being asked. So `khalid serve` also switches huge pages off for its own process before it reads the site (`prctl(PR_SET_THP_DISABLE)`, through `rustix`, which wraps it safely). `khalid-perf serve` prints the system's huge-page mode beside the trace.

`khalid-perf serve` prints, beside its measures and so in CI's log, the resident memory each second of the 15-second wait and the split at its end: anonymous memory, the part of it in huge pages, and code and mapped files.

What a settled server holds, from `/proc/<pid>/smaps` at 51 MB: 27 MB in mimalloc's arena, 16.4 MB of the binary's code, 5.3 MB in the C allocator's heap, where SQLite keeps its page caches, and 1.4 MB of the C library.

## Fast to build

- `sccache` and `mold` in CI. `cargo nextest` with the default partitioning.
- `cargo check` on every push, the full suite on pull requests.
- `cargo bloat --release --crates` in the job summary, so a heavy dependency is seen the day it arrives.

## Baseline

`perf/baseline.json` holds the last accepted numbers and the fixture version. `scripts/perf.sh` runs every measure that can run on a CI runner and writes `perf/current.json`. The gate compares the two.
