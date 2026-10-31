# The visitor log: privacy design

This note is for the site owner and their lawyer. It says what the visitor log keeps, what it never keeps, and for how long. The log is off on every site until `site.toml` switches it on (decision D5 of `docs/decisions/2026-10-28-geo-build-list.md`).

## Why it exists

The page counts Khalid already keeps (KH-GRO-4) hold no address and no user agent, so they cannot say whether a visitor calling itself Googlebot really is Google, or how long a new post waited for its first crawl. The visitor log keeps one row per visit for a short time, so later features can answer those questions: the first-crawl timer, crawl health, crawl alerts and the bot error report (C2 to C5), and AI visitors per tool (M1).

## Switching it on

```toml
[visitor_log]
on = true       # off unless you write this line
keep_days = 14  # how long each row is kept: 7 to 14 days
```

A static site cannot switch it on: the pages are sent by another host, so Khalid sees no visits.

## What one row holds

| Field | Example | What it is |
|---|---|---|
| Time | `2026-10-31T09:14:05Z` | When the visit came, to the second, in UTC |
| Page | `/blog/what-is-geo/` | The address asked for, with no query string |
| Answer | `200` | The HTTP status the page was answered with |
| Shortened address | `203.0.113.0` | The visitor's network address with its end set to zero (see below) |
| Visitor code | `9f3c2a71d05be448` | A code that groups one visitor's rows on one day (see below) |
| Bot | `GPTBot`, or empty | The bot the user agent claimed to be, empty for a person's browser |
| Bot checked | `yes`, `no`, `no list` | Whether the full address was in the list that bot's company publishes |
| Came from | `search`, `ai`, `social` | The group the referring site belongs to, never its full address |

## What is never kept

- No cookie or other storage in the visitor's browser. The log works from what every request already sends.
- No full network address. It is shortened before anything is written.
- No user agent text. Only the bot name from a fixed list, or nothing.
- No query string, form content, referring address or sign-in identity.

## Shortened addresses

An IPv4 address keeps its first three parts and ends in zero: `203.0.113.57` becomes `203.0.113.0`, shared by up to 256 addresses. An IPv6 address keeps its first 48 bits, the part that names a network rather than a device: `2001:db8:85a3:8d3:1319:8a2e:370:7348` becomes `2001:db8:85a3::`.

## The visitor code

When Khalid starts, and again at midnight UTC, it makes a new random key of 32 bytes. The key lives only in the server's memory: it is never written to disk, logged or sent. Each visit's code is the first 8 bytes of a SHA-256 hash of the key, the full address and the user agent.

The same visitor gets the same code for the rest of that day, so a report can tell ten visits from one person apart from ten people. At midnight the key is thrown away and a new one made, so the next day's codes cannot be linked to today's, and nobody, the site owner included, can turn a code back into an address. A restart also makes a new key, which only splits a day's codes in two.

## How long rows are kept

Each row is deleted once it is older than `keep_days` days, 14 unless set, never more than 14 and never fewer than 7. Deletion runs with the writes, at least once a day while the server runs, and when it starts.

## Checking a bot's claim

Google, Bing, OpenAI and Perplexity publish the network addresses their crawlers use (decision D2). Khalid fetches those lists from the companies' own sites, keeps them in `data/`, and refreshes them once a day. The request sends nothing about any visitor. A visit that claims a bot's name is checked against the full address in memory, before it is shortened, and only the answer is written.

Anthropic, Apple and most other bots publish no list this version knows of, so their rows say `no list`.

## Where it lives

In `data/site.db`, the site's own database file, on the site owner's server. It is never sent anywhere by Khalid. The table is `visitor_log`.
