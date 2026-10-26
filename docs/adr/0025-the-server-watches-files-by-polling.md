# 0025. The server watches the site folder by looking, not by file events

**Status:** Accepted, 2026-10-26. Phase 1 task 1.15.

## Context

Task 1.15 asks for file watching, so an edited post appears without a restart. `docs/standards/rust.md` names `notify` for this. Every release of `notify`, up to 9.0, is licensed CC0-1.0. CC0 puts the code in the public domain, so it is as free as a licence can be, but it is not on the allow list in `deny.toml` (BL-DEP-2), and a track may not change that list. Native file events also fail quietly in some places writers run Khalid: Docker volumes on macOS, network drives, and some virtual machine shares.

## Decision

- **The server looks at the folder every half second** and compares each file's size and change time with the last look. When anything was added, removed or changed, it renders the pages again and swaps them in whole.
- **Hidden files and folders, `data/` and `public/` are left out,** so `.git`, editor swap files, the database and the static export never cause a rebuild. The server's own writes can never start a loop.
- **The walk runs on a blocking thread** (BL-RUN-7) and does not follow links, so a link to a folder above cannot make it loop.
- **A render that fails is logged and the last good pages stay.** The next save tries again.

## Consequences

- An edit shows within about half a second plus the render, which is quick enough for a writer switching to the browser.
- Each look reads the metadata of every file. For 1,000 posts with their images, that is a few milliseconds every half second on one blocking thread. Task 1.21 measures idle memory and CPU with the 1,000-post fixture.
- Switching to `notify` later, if the founder allows CC0-1.0, changes only `watch.rs`. The question is in the phase brief.
