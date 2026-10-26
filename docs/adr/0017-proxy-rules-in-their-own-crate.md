# 0017. Proxy rules live in their own crate and keep the mount path

**Status:** Accepted, 2026-10-26. Phase 3 work pulled forward, task 3.6.

## Context

Plan section 4.8 asks for the proxy rule that mounts the blog under a folder of a site built with something else, for ten stacks. `docs/standards/rust.md` gave this to `khalid-theme`, but writing a config snippet shares nothing with reading a site's colours, and that crate is still in review.

## Decision

- **A crate of its own, `khalid-mount`,** on `khalid-core` and `thiserror` only: the rule for a stack, a mount path and the address Khalid answers on.
- **The path is passed on unchanged.** Khalid knows its mount path and writes every link with it, so no stack strips the prefix.
- **The mount path is checked before it reaches a config:** a leading slash, no trailing slash, no empty or dot folders, and only letters, digits, `-`, `_`, `.` and `~`, so no quote or brace can break out of a snippet. The root is refused, because a blog at its own address needs no rule (plan section 1, decision 1).
- **Webflow and Shopify use the Cloudflare Worker,** because neither can pass a folder to another server (plan section 4.8). They share its snippet and its fixture.
- **One fixture per snippet,** matched byte for byte, so any change to a rule is a visible change in review.

## Consequences

- `khalid init` and `khalid doctor` call one function. A new stack is a new variant, a new arm and a new fixture.
- A stack we do not list gets an error naming every stack there is.
