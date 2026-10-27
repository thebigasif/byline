# 0000. The site audit crawls the blog like a search engine, from any source

**Number:** next free. **Status:** Proposed, 2026-10-27. Audit 2, brief G30 (KH-LNT-5, plan section 5.10).

## Context

KH-LNT-5 asks for `khalid audit`: crawl the site the way Ahrefs and Semrush do, run their checks in their categories, score the site's health with a trend, give a fix for each issue and a CSV export, and write the internal linking report with click depth and link rank. The linter already checks posts as files. The audit has to see what a crawler sees: status codes, redirects, the head of each page, the links between them. The brief says it may crawl the built site or the served one.

## Decision

- The audit lives in its own crate, `khalid-audit`. It knows nothing about content files, rendering or storage. It sees the site only as a crawler does.
- A crawl asks a `Source` for one address at a time. A source never follows a redirect, so the crawl sees each hop. The built folder, a running server and the tests' tables are three sources behind one trait; the checks never know which one answered.
- The crawl is breadth first from the blog's home page, so each address is found at its fewest clicks, and that count is its click depth. A redirect's target gets the redirect's depth.
- Inside the blog means the home page's scheme, host and port and a path under the home page's path. Every address inside is fetched once: pages, and the stylesheets, scripts and images they load. An address outside, including the main site around a mounted blog, is recorded and checked for dead links, never crawled.
- Each HTML page is read once, in one walk over the parsed document with an explicit stack, into the fields the checks need. The body is not kept.
- A crawl stops at 50,000 addresses by default and says so, so a site with endless generated addresses cannot run forever.

## Consequences

- One set of checks serves `khalid audit` on a built folder in CI, on a running server at night, and in tests without a network.
- The crawl asks for one address at a time. On a folder that is fast; over HTTP it is slower than a parallel crawler, and the HTTP source can add parallel requests later without changing the checks.
- Pages that nothing links to are not crawled from links. The sitemaps add them, which is how the orphan check finds them.
