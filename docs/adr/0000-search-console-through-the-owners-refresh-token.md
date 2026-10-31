# 0000. Search Console through the owner's refresh token, read on demand

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase 5, brief G43, KH-GRO-5.

## Context

Plan 5.11 and decision D-8 ask for an opt-in Search Console connection with the owner's own Google account: per-post clicks, impressions, position and click-through rate, index status, winners and losers every week, decay over three months, 404 and noindex addresses that still get search traffic, a weekly email, and "Go deeper in SerpDad" links. The gap audit (G43) asks for it "with a stub API". The admin screen for it needs a mockup and the founder's yes, so this task builds the connection, the numbers and the commands, not the screen.

Google's Search Console APIs sign in with OAuth 2.0 only. Two ways fit a server: a service account, which signs its own JWT with an RSA key, or an OAuth client whose refresh token the owner granted once. Search Analytics (`searchAnalytics.query`) keeps about 16 months of data. The URL Inspection API answers one address per call, within a quota of 2,000 calls a day and 600 a minute per property.

## Decision

- **An OAuth client and a refresh token, not a service account.** A service account needs RSA signing in the binary and the owner to add a robot user to the property. The refresh token is the owner's own account, as D-8 says. `[search_console]` in `site.toml` names the client ID, the property (the site's address unless set) and who gets the weekly email. The client secret and the refresh token come from `KHALID_GOOGLE_CLIENT_SECRET` and `KHALID_GOOGLE_REFRESH_TOKEN`, held in `Secret` and hidden from `Debug` (BL-RUN-5). Getting the refresh token is a step in `docs/deploy.md` until the admin's connect button exists.
- **A crate of its own, `khalid-search-console`.** It holds the client, which sends through the `Outbound` it is given (BL-RUN-8), and the pure arithmetic: windows of days, per-post numbers, winners and losers, decay, and the weekly text. It knows nothing about files or the command line.
- **Read on demand, nothing stored.** Every number is asked of Google when a command runs. Google keeps 16 months, so decay over three months and week-on-week changes need no table of our own, and nothing derived is written into `content/` or `data/`.
- **`khalid search-console` is the door for a person**, as `khalid bing` is: `report` for the numbers, `index` for index status within the quota, and `email` for the weekly report, run weekly by a timer the owner sets up.

## Consequences

- No history beyond Google's 16 months. A stored history can come later as a table in `data/site.db` if the admin needs charts older than that.
- The weekly email leaves only when something runs `khalid search-console email` each week. `khalid serve` does not schedule it yet.
- A refresh token the owner revokes, or one Google lets lapse, gives a plain message that says how to make a new one.
