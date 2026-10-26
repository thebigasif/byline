# Architecture decision records

One file per decision the plan does not already make. Numbered, never edited after acceptance; a new decision supersedes an old one and says so. Format: context, decision, consequences, each in plain words. Rule BL-DOC-3.

**Numbering.** Take the next free number from `main` when you open the pull request, re-check it right before the merge, and if two records still collide the later merger renumbers. Accepted records are never edited, but a file may be renamed to fix its number; the title changes with it.

| Number | Title | Status |
|---|---|---|
| 0001 | Content is files, the database is for the rest | Accepted |
| 0002 | One binary, SQLite, no external services | Accepted |
| 0003 | The API is the only door | Accepted |
| 0004 | Errors reach people as what, why, fix | Accepted |
| 0005 | Permissive dependencies only, so SerpDad can reuse the crates | Accepted |
| 0006 | Where the command line writes, and the shape of its JSON | Accepted |
| 0007 | TLS through rustls with the ring provider | Accepted |
| 0008 | YAML front matter, read with serde-saphyr | Accepted |
| 0009 | Render input records and the template sandbox | Accepted |
| 0010 | The database: derived tables, visitor tables, one writer | Accepted |
| 0010 | Webhook event names and signatures | Accepted |
| 0011 | The lint engine reads its own input model and one message catalogue | Accepted |
| 0011 | The lint engine reads its own input model and one message catalogue | Accepted |
| 0012 | Blocks beyond markdown are fenced directives | Accepted |
| 0013 | SEO outputs are built from their own input types | Accepted |
| 0014 | Webhook event names and signatures | Accepted |
| 0016 | The share image is drawn in pure Rust, in its own crate | Accepted |
| 0017 | Proxy rules live in their own crate and keep the mount path | Accepted |
| 0015 | Theme sync reads the site it is given, and only that site | Accepted |
| 0018 | Blocks with no body are one-line leaf directives | Accepted |
