# 0002. One binary, SQLite, no external services

**Status:** Accepted, 2026-10-26. From the founder's plan, sections 4.1, 4.5, 4.7.

## Context

Ghost needs Node and MySQL 8 in production, Mailgun for bulk email and Tinybird for analytics. WordPress needs PHP-FPM, MySQL and a web server. Every extra service is an install step, a cost and a failure mode, and the plan's promise is one file to install.

## Decision

Byline is one statically linked binary. It embeds the admin, the default templates, the schema.org vocabulary and the lint rules. It stores everything in files and SQLite. It sends email itself through SMTP or a provider API. It counts analytics itself. It obtains and renews its own TLS certificate. Postgres, Redis, a message queue, a search service or an analytics service are not options in 1.0.

## Consequences

- The binary must stay under 30 MB and idle under 50 MB (performance budgets).
- SQLite runs in WAL mode with one writer and a read pool (BL-DB-1). Heavy work goes to a worker pool, not a separate service (BL-RUN-7).
- Media may live on an S3-compatible bucket as an option, because disks fill up. That is storage, not a service the binary depends on to start.
- Scaling out is not a 1.0 goal. One site on one small server is the target.
