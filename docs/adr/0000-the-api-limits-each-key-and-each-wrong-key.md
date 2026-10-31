# 0000. The API limits each key, and each address that sends wrong keys

**Number:** next free. **Status:** Accepted, 2026-10-31. Audit 2, brief G33 (KH-OPS-5, RULES.md BL-SEC-5).

## Context

BL-SEC-5 asks for rate limits on the API "per IP and per key, with a plain message when hit". Until now only the comment form had one, in `khalid-comments`. The Admin API checks an integration key or staff token against an Argon2id hash, which takes real work on purpose, so a flood of made-up keys could keep the server busy. The plan gives no numbers, and another track is adding Admin API routes at the same time, so the limit has to live where every route already passes: the two key checks.

## Decision

- **One limiter, `khalid_api::api_limits::ApiLimits`, shared by both key checks.** `require_admin_key` and `require_content_key` call it, so every route behind them, present and future, is limited without a change to its own file.
- **Per key, once the key is accepted:** 600 Admin API requests a minute for an integration key, staff token or staff session, and 6,000 Content API requests a minute for a content key. A content key is shared by every reader of a headless front end, so it gets ten times more. A rejected request is not counted.
- **Per address, for wrong keys:** 10 a minute. An address with none left gets the 429 before its key is checked, so made-up keys stop costing Argon2id work. A request with no key at all costs nothing and is not counted, so a browser that opens the API by hand is never paused. The address is the peer's, or the last `X-Forwarded-For` entry when the peer is a proxy on this machine or the local network, the same rule the comment form uses. IPv6 counts by /64.
- **Each limit lets its minute's count through at once and earns one back each sixtieth of a minute** (the generic cell rate algorithm). Each bucket keeps one time, when it is full again, so memory stays small; at most 50,000 buckets are kept, and full ones are forgotten first.
- **The answer is a 429 `too-many-requests` problem with `Retry-After` in whole seconds**, and the fix in plain words says how long to wait.
- **The limits are constants, kept in memory.** A restart empties them. A `[limits]` table in `site.toml` can come later if a site needs other numbers; the config type lives in `khalid-core`, which another track owns.

## Consequences

- A site behind a proxy that is not on the local network, such as a CDN, sees every request from the proxy's address. Ten wrong keys from one client then pause everyone behind that proxy for a few seconds. Sites like that should forward from a local proxy, as the `khalid init` proxy rules do.
- The addresses are held only in memory and only until their limit fills up again, at most a minute, so they are not hashed as the comment form's day-long counts are.
- Two Khalid processes behind one balancer each keep their own counts.
