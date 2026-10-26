# The build writes its redirects as a `_redirects` file

Number: next free

**Status:** Accepted, 2026-10-26. Phase 1 tasks 1.13 and 1.16.

## Context

A post's front matter can list `aliases`: old addresses that must now lead to the post, so a moved post keeps its links and rankings. `khalid-seo` turns redirects into a checked table, with chains collapsed into one hop and loops refused (task 1.13, part 13a). `khalid serve` answers redirects itself, but `khalid build` makes a folder of static files, and a static host cannot run code. The redirects have to reach the host as a file it reads.

## Decision

`khalid build` writes `_redirects` into the output folder: one line per rule, `from to status`, with the chain already followed, so every line is one hop.

- The format is the one Netlify and Cloudflare Pages read, and the one most static hosts copy. Each path starts at the site root and includes the mount folder, such as `/blog/old-title/ /blog/why-internal-links-matter/ 301`.
- Aliases make only permanent moves, 301. Gone (410) and legally blocked (451) addresses have no setting in the content yet, so the file holds moves only until they do.
- An alias must start with the mount folder. One outside it is refused with the file and the alias named, because the blog cannot answer for the rest of the site.
- `khalid serve` reads the same table from the content, never from this file.

## Consequences

- Netlify and Cloudflare Pages honour the redirects with no setup. Other hosts need the lines copied into their own config; the proxy snippets of task 3.6 can grow a redirect section later.
- Two posts claiming one alias, or an alias that is a post's own address, stop the build with a plain message, as the table refuses them.
