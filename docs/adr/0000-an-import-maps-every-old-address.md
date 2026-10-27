# An import plans first and maps every old address

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 6 task 6.1, pulled forward into `docs/phases/phase-1.md`.

## Context

A blog that moves from WordPress must keep its rankings, so every address that search engines and readers know has to keep answering. KH-API-7's done line is "a 500-post WordPress import loses nothing": zero lost posts, every slug kept and a complete redirect map. Plan section 5.7 also asks for a dry run first and a report of what came in and what did not.

Three facts shape where redirects can live:

- A post's `aliases` front matter makes Khalid answer an old path itself (ADR 0031). But `Routes::redirect` refuses an alias outside the blog's mount folder, because Khalid never sees those requests.
- Old WordPress addresses usually sit at the root of the site, such as `/2024/03/rye/`, while the blog is often mounted at `/blog/`. Sometimes the blog moves to a new domain too.
- The Redirection plugin keeps its rules in its own table, and a WordPress export never holds them.

## Decision

- **Plan, then write.** `wordpress::plan` works out every file an import would write, and writes nothing. The dry run and the real run see the same plan. Every content file in the plan is read back with `khalid-content`'s own types before it is planned. An item whose file would not read back is listed as failed with the reason, never written.
- **Keep slugs.** A WordPress slug is kept whenever it fits the site's slug rules. One that breaks them is made again from the old slug, and the change is recorded for the report.
- **Map every old address once.** `Plan::redirects` holds one entry per address the old site answered, in this order:
  - the Redirection plugin's rules, from its JSON export, because they answered before WordPress did;
  - every published post and page;
  - every image's own page, which leads to its post;
  - every category archive.

  An address that stays the same is listed as kept, so the map is complete, not only a list of moves. Targets are full addresses, so the map also works across a change of domain. A rule that pointed at a post that moved leads straight to the new address, so no chain is left.
- **Two outputs for the map.**
  - When the old site and the blog share a host and an old post address lies inside the mount, the post's front matter keeps it in `aliases`, and Khalid answers it.
  - Every entry that redirects, inside the mount or not, goes into `import/wordpress-redirects.csv`, with `from,to,status,match` columns and `exact` or `regex` in the last. The host's rules or the redirect manager's CSV import (KH-SEO-8) read it.
- **What front matter cannot hold is held, not dropped.** Each imported item lists what it had and the files cannot hold yet, such as tags, a search title or a feature image, so the report names it.

## Consequences

- Nothing is lost silently: every item is imported, failed with a reason, or left behind with a reason, and every old address has an entry.
- `import/` is a record of one import, outside `content/` and `data/`. A person may delete it once the redirects are in place on the host.
- Tag archives get map entries once content has tags (the open question in the phase brief).
- The Medium importer follows it too, with one difference: a Medium post's old address may be on medium.com, on a publication's own domain, or on the very host the blog moves to, and each post also answers at Medium's short address `https://medium.com/p/<id>`. So its map holds whole old addresses, each once, and `import/medium-redirects.csv` writes an address on the new site's host as its path and any other address whole. An old path inside the blog on the new host also goes into the post's `aliases`. Only Medium can answer a medium.com address, and it does not redirect, so the report counts those entries: the person keeps them as a record, and Medium's own settings are the only way to point readers there to the new home. Medium's slug is the readable part of the address without the id; a post whose slug an earlier post took keeps Medium's whole last part, id and all.
- The Ghost importer follows the same decision: a plan first, kept slugs, one entry per live post, page and author archive from Ghost's default routes, and `import/ghost-redirects.csv` with the same columns. A Ghost export does not name its site's address or its custom routes, so the old address is given to the plan.
