# 0000. `khalid serve` hands freed memory back once its work is done

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 1 task 1.21.

## Context

The idle memory budget is 55 MB on the 1,000-post site, measured 15 seconds after the server's first answer (CTO answers 2026-10-27, section 2). Every pull request measured about 73 MB in CI. A trace of the server's resident memory every half second, on this server, showed why:

- The start renders every page, compresses them, and builds the Content API's store and index. Resident memory peaks above 100 MB and then sits at 70 to 80 MB, although a `dhat` profile at 14 seconds holds only about 16 MB of live data.
- The plateau ends all at once, at about 10.5 seconds, never before. That is when tokio's idle blocking threads end, 10 seconds after their last task. Without them, it would not end at all.

mimalloc, the binary's allocator, returns a freed page to the system one second after it is freed, and only when some thread later frees a page or ends. An idle server does neither. A blocking thread's own pages are only collected when that thread ends. On a slower runner the start's work ends later, so the drop lands after the 15-second reading, which is the 73 MB CI kept reporting.

mimalloc's purge delay can be set only through environment variables or `unsafe` calls, and the workspace forbids `unsafe`.

## Decision

- **Blocking threads end one second after their last task**, not ten (`thread_keep_alive`). Their pages are collected soon after a render instead of ten seconds later.
- **Two seconds after heavy work, the server ends one short-lived thread** (`crates/khalid/src/memory.rs`). Heavy work is a replacement of the served pages, which every full render, saved post and Brotli pass ends with, or the Content API's feed finishing a catalog. Two seconds is past mimalloc's one-second delay, so everything freed by then is returned. Work during the wait starts it again, so a burst of saves ends one thread, not one per save.
- `khalid-server` gives `LivePages::replacements` for this. It holds no pages.

## Consequences

- Measured on the generated 1,000-post site, alternating runs on this shared server: idle memory 64.35, 58.61 and 63.91 MB with the hand-back alone; 56.07, 52.48 and 51.36 MB with the one-second keep-alive added. Main, in eight runs on the same server, measured anything from 51.43 to 81.49 MB, depending on whether the threads ended before the reading. The trace shows memory falling about two seconds after the feed ends instead of at ten seconds.
- A save after a quiet second starts a new blocking thread, which costs tens of microseconds next to a render of milliseconds.
- The idle figure now depends on when the start's work ends. The Content API's feed, which writes the index and the search index, takes 5 to 11 seconds here under load, and its working data is live until then.
- If mimalloc's options become settable without `unsafe`, a shorter purge delay would replace the extra thread.
