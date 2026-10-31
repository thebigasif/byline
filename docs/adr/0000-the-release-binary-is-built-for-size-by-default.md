# 0000. The release binary is built for size by default, and the reader's path for speed

**Number:** next free. **Status:** Proposed, 2026-10-31. Task p1-binary-size, KH-OPS-4, BL-PERF-1.

## Context

The release binary must stay under 30 MB (`docs/standards/performance.md`). Integration branches 15 and 16 built 33.2 MB and 31.6 MB, and 33.8 MB together. The release profile already used fat LTO, one codegen unit, stripped symbols and `panic = "abort"`, and built a few cold crates at opt-level `"s"`, one entry per crate. Every new feature crate started at opt-level 3, so each track that added code moved the binary towards the budget, and the next fix was another list of crates to shrink.

Most of the binary is code that never runs while a reader waits: the commands, the admin and the APIs, mail, imports, image uploads, HTTPS certificates. A small set of crates carries the reader's path and the build: the server stack, the allocator, rendering, the build, the linter, and the parsers and compressors they use.

## Decision

- **`opt-level = "s"` for the whole release profile.** A new crate is sized by default.
- **opt-level 3 for the reader's path and the build's path,** named one crate per entry in `Cargo.toml` with the reason above the list. A crate joins that list when a budget measured by `scripts/perf.sh` shows it is needed.
- **The `khalid` binary crate stays at `"s"`.** Fat LTO optimises the whole program again at the binary crate's level, and opt-level 3 there made the binary 35.3 MB, bigger than before.
- **The panic behaviour does not change.**

## Consequences

- Both integration branches merged build 27.5 MB instead of 33.8 MB, with about 2.5 MB of room for growth.
- An idle server holds about 8 MB less memory, because less code is mapped in.
- The first byte after a cold start is about 3.5 ms slower (77 ms against 74 ms, budget 100 ms). Render and rebuild times did not move beyond the noise between runs.
- A new crate on the reader's path is slower until it is added to the opt-level 3 list. The perf gate is what catches it.
