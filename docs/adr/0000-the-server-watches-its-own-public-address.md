# 0000. The server watches its own public address for speed and uptime

**Number:** next free. **Status:** Proposed, 2026-10-31. GEO task C6, KH-GRO-6.

## Context

The GEO build list (`docs/phases/phase-geo.md`) names C6, "speed and uptime watch", with no brief and no done line. Neither Steve Toth nor Koray Tugberk Gubur, the only sources for SEO and GEO behaviour (`docs/decisions/2026-10-28-geo-build-list.md`), gives a speed threshold or an uptime rule, so this task builds no SEO advice. It measures plain facts: whether the blog answered and how long it took, and tells the owner through the alerts that already exist (G44).

## Decision

- **`khalid serve` looks at the blog's home page at its public address**, the `address` and `mount` in `site.toml`, every `[alerts] watch_minutes` (5 unless set, 0 switches it off, at most 60). It asks through the one shared `Outbound` client, so DNS, the certificate and any proxy or CDN in front are on the way, as they are for a reader. One page keeps the cost to one request every few minutes; a slow home page is the case owners notice first.
- **The watch is off while the server answers only this computer**, the default `127.0.0.1:8080` while writing, because the address in `site.toml` is then not this server. It runs with `--domain`, or when `--listen` is not a loopback address, as in a container or behind a proxy. An address the private-network guard refuses is logged and not watched.
- **Each look is kept for 30 days** in a new visitor table, `site_watch`: the page, the time, the status or none, and the milliseconds until the whole answer was read or the look gave up (10 seconds, `Outbound`'s limit).
- **Two alert kinds join `alert.raised`.** `site-down`: the last three looks all failed, which is no answer, or a status outside 200 to 399. `slow-site`: the middle answer time of the last hour's good looks is over `[alerts] slow_ms` (1000 unless set, 0 switches it off), with at least three looks. The middle, not the average, so one stalled look does not raise it. Both clear when the site recovers, so the next outage is raised again.
- **`khalid watch` prints the record**: for the last 24 hours and the last 30 days, the looks, the share answered, the middle and slowest times, and the longest gap between looks, with `--json` for scripts.

## Consequences

- A server that is down cannot report itself. Its looks stop, and `khalid watch` shows the hole as the longest gap. An owner who needs a page at 3 a.m. still needs a second machine, or an outside monitor, watching the address.
- A blog whose public address loops back through the same machine (hairpin NAT) may not reach itself; the looks then fail and `site-down` is raised. Setting `watch_minutes = 0` silences it.
- The slow check needs three good looks in an hour, so it never fires with `watch_minutes` above 20. The down check still works up to 60.
- The watch adds one outbound request every 5 minutes and one row per look, about 8,600 rows per month.
