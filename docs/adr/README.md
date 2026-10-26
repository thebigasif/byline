# Architecture decision records

One file per decision the plan does not already make. Numbered, never edited after acceptance; a new decision supersedes an old one and says so. Format: context, decision, consequences, each in plain words. Rule BL-DOC-3.

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
| 0009 | Webhook event names and signatures | Accepted |
