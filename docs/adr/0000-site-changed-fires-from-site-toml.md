# 0000. `site.changed` fires from a saved `site.toml`

**Number:** next free. **Status:** Proposed, 2026-10-31. Audit brief G25, KH-API-3.

## Context

The event catalogue (ADR 0014) has `site.changed`, "site settings changed". The settings live in `site.toml` (ADR 0001: files are the source of truth). They change when someone saves that file by hand, through `git pull`, or, once G24 lands, through the Admin API's settings routes, which write the same file. `khalid serve` reads its settings once at start, so a change takes effect on the next start.

## Decision

- **`khalid serve` compares `site.toml` with each render, like the entries (the ADR on entry webhooks).** Each render's catalog reads and parses the file; the Content API's task compares it with the last good one and fires `site.changed` when a top-level setting or table was added, changed or removed. Comments and spacing do not count.
- **The body names the settings, never their values:** `{"settings": {"file": "site.toml", "changed": ["mail", "seo"], "restart_needed": true}}`. A value such as a mail sender or a verification code may be private, and the receiver can read the file or the Admin API for the rest.
- **A broken file fires nothing.** A save that is not valid TOML, such as one caught half written, is skipped; the next good save is compared with the last good one.
- **The Admin API settings routes should not fire it themselves.** They write `site.toml`, so the comparison fires once.

## Consequences

- `restart_needed` is always `true` until the server reloads its settings while running.
- A save made while `khalid serve` is stopped fires nothing on the next start.
- Other files that shape the site (`categories.toml`, theme tokens) do not fire `site.changed`; categories have their own routes in G24.
