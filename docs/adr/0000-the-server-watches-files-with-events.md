# The server watches the site folder with file events, and looks only where it must

**Number:** next free.

**Status:** Accepted, 2026-10-27. Phase 1 task 1.21. Supersedes "the server watches the site folder by looking, not by file events".

## Context

`khalid serve` looked at every file of the site every half second. On the 1,000-post site a saved post waited 250 ms on average before anything happened, five times the 50 ms budget for a saved post to show (`docs/standards/performance.md`). The CTO answers of 2026-10-27, section 1, allow `notify`, whose licence CC0-1.0 is public domain, for this crate only, and keep looking at the folder as the fallback where file events are not available.

## Decision

- **`watch` asks the system for file events** on the whole site folder through `notify` (inotify on Linux, FSEvents on macOS, ReadDirectoryChangesW on Windows). A change renders the pages again after 10 ms, so the several events of one save cost one render.
- **Where events cannot be had, `watch` falls back to `poll`**, the half-second look from the record it supersedes, and logs once that it did. `poll` stays public, so it can be tested on its own.
- **The same files are left out** either way: hidden files and folders, and `data/` and `public/` at the top of the site. Reading a file is not a change, since every render reads them all, so read events never start a render.
- **An error or a lost event counts as a change**, because it might hide one. The cost is one render more than needed.
- **Setting the watch up walks the folders on a blocking thread** (BL-RUN-7), and the folder is compared once with the look taken before the first render, so a file saved while the server started is still seen.

## Consequences

- A save reaches the renderer in about 10 ms instead of 250 ms on average, and an idle server does no work at all where it used to read every file's details twice a second.
- The server holds one inotify watch per folder of the site. Linux allows 8,192 per user by default and more on most distributions; a site has far fewer folders.
- Network drives and some container volumes give no events. There the server still shows a save within half a second.
