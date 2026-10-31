# 0000. Form blocks post under the mount path, set after the page is rendered

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase 1, G4 forms in server mode (KH-GRO-1).

## Context

The form block in `khalid-content` writes `action="/forms/<id>"`. On a blog mounted at `/blog/`, the proxy sends only `/blog/` to Khalid, so that address never reaches the server. The G4 brief says the action must include the mount path. The content renderer does not know the mount, and `khalid-content` belongs to the content track.

## Decision

On `khalid serve` in server mode, the binary rewrites each form block's opening tag after the page is rendered, in `crates/khalid/src/form_pages.rs`, the same way static mode locks forms after render (ADR "static mode locks forms after the page is rendered"). The same pass notes which page holds each form id, and the route answers only those ids. It runs beside `comment_pages::fill` on every render, so a saved post gets its forms right away.

## Consequences

- `khalid-content` stays unaware of the mount, and its snapshots do not change.
- The rewrite matches the block's exact opening tag. If the block's markup changes, `form_pages::tests::each_form_block_posts_under_the_mount` and `crates/khalid/tests/serve_forms.rs` fail and point here.
- A form taken off a page stays known until the server restarts. Posting to it still keeps the message, which is harmless.
- `khalid build` in server mode still writes `/forms/<id>`. A server-mode site is served by `khalid serve`, which renders its own pages. If the content track later passes the mount to the renderer, this pass can go.
