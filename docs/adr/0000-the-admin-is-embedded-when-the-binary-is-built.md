# 0000. The admin is embedded when the binary is built

**Number:** next free.
**Status:** Proposed, 2026-10-27. Phase 4, the admin app, part 5.

## Context

Plan section 4.6 asks for the admin to be built once and embedded in the binary, served under `/admin` by `khalid serve`. The admin is built by Vite with npm. The Rust jobs in CI, and anyone running `cargo build` from a fresh clone, have no `admin/dist/`.

## Decision

- **A crate of its own, `khalid-admin`,** holds the built files and the routes that answer them. Its build script lists every file in `admin/dist/` into `OUT_DIR` with `include_bytes!`, so the binary carries them and reads nothing from disk. No new third-party crate: the routes use the `axum` the server already uses.
- **When `admin/dist/` is missing, the build still works.** `/admin/` then answers 503 with a page saying the admin is not in this copy and how to add it. The blog and its APIs work either way.
- **Addresses.** `/admin` moves to `/admin/`. A built file answers as itself, and files under `assets/`, which Vite names by a hash of their content, are cached for a year. Any other address under `/admin/` gets `index.html`, whose router shows the screen. A missing file under `assets/` is a 404, so a stale script is never answered with a page.
- **Policy.** Every admin answer carries its own Content-Security-Policy: scripts, styles, images and API calls from the blog itself only, no framing, no base element. The public pages keep their nonce policy (ADR 0023).
- **The order of the build** is `npm ci && npm run build` in `admin/`, then `cargo build`. The release workflow needs that step before it builds binaries; that file belongs to the CTO.

## Consequences

- A release binary is one file with the admin inside, as section 4.1 says.
- A binary built without npm says so plainly at `/admin/`, instead of failing to build.
- The admin is always at `/admin/`, even when the blog is mounted under a folder. Making the path configurable means building the admin with that base, and is left for when Settings can change it.
- The files are answered as built, not compressed. Precompressing them in the build script is a later change if the transfer size matters.
