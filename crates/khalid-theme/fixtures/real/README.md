# Real stylesheets

Stylesheets saved from real sites, so theme sync is tested on CSS nobody wrote for it. `tests/real_sheets.rs` checks that each gives the colours and fonts a person sees on that site. The files under `fixtures/sites/` are shaped like real sites but written by hand.

Each file is kept as it was published, under its own permissive licence:

| Folder | Source | Licence |
|---|---|---|
| `nodejs-api/` | The Node.js API documentation, `doc/api_assets/style.css` (as installed in `/usr/share/doc/nodejs/api/assets/`), its first 149 lines: the variables, the page and the links. The rest styles the side menu and code | MIT, Copyright Node.js contributors |
| `istanbul/` | The HTML coverage report of istanbul-reports, `lib/html/assets/base.css` (as installed in `/usr/share/nodejs/istanbul-reports/`), whole | BSD-3-Clause, Copyright Yahoo! Inc. |
| `bzip2/` | The bzip2 1.0.8 manual, `bzip.css`, whole | The bzip2 licence, a BSD-style licence, Copyright Julian R Seward |

The comrak website's stylesheet (BSD-2-Clause) was considered and left out: its `:root` sets `color-scheme: dark`, so the site shows its dark colours, and the blog is light by default.
