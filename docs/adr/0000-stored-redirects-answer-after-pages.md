# 0000. Stored redirects answer after pages and aliases

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 1 audit brief B-SEO-8, KH-SEO-8.

## Context

ADR 0010 keeps redirects people make, imports bring and slug changes add in the visitor table `redirects`, and ADR 0031 turns the `aliases` in front matter into moves that `khalid serve` answers from its pages in memory. With both in place, one address can have a page, an alias and a stored rule at once, and the server must answer in one order every time. A stored rule can also be a regular expression, which may catch addresses nobody meant it to. The server answers from memory and must not read the database per request (ADR 0024).

## Decision

- **A page answers first, then an alias, then a stored rule.** The stored redirects are a layer over the page router that acts only when the router's answer is a 404 for a `GET` or `HEAD` under the mount folder, never for `/api/`. A live page is never hidden by a stored rule or a broad pattern, and a post that comes back simply answers again.
- **The checked table lives in memory.** `khalid serve` builds a `RedirectTable` from the rows at start, and every write through the Admin API rebuilds it, so a new redirect answers at once and a request never waits for SQLite. Rows that make no table, such as two writers racing into a loop, leave the last good table served and a warning in the log.
- **One step for the reader.** A chain of stored rules is already collapsed by the table. When a stored rule ends at an address an alias has moved, the answer goes straight to the alias's page. The query string is kept on moves to a path.
- **Every other 404 under the mount is logged, off the request path.** The layer hands the address to one writer task through a queue of 256; when the queue is full, or the address is over 512 bytes, it is dropped. The log itself holds 10,000 addresses. A 404 under `/api/` or outside the mount is never logged, because no reader asked the blog for it. The last few 404s before a stop may not be written.
- **The answer is the 404 turned into a redirect.** It keeps the security headers the page router set. A move has an empty body; a 410 or 451 keeps the not-found page as its body, so a reader still sees the site.

## Consequences

- A stored rule for an address that has a page does nothing until the page goes; the debug view says so, and the API refuses nothing for it. This is the safer of the two mistakes.
- Static hosts get only what `khalid build` writes into `_redirects`; stored rules reach a static site through the CLI part of this track.
- The admin's redirect screen, when it is built, shows stored rules and the debug view; aliases stay in their posts.
