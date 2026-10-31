# 0000. One draft model: a post is live only in `content/` with a date

Number: next free

**Status:** Accepted, 2026-10-31. 1.0 readiness check, findings 3 and 9.

## Context

Two rules said what a draft is, and they disagreed.

- `khalid build`, the linter, feeds, search and the sitemap leave out a post in `content/<language>/posts/` with no `date:`. The build's refusal told users to remove `date:` to keep a post as a draft.
- The publish door (ADR 0030) counted only `drafts/<language>/<slug>.md` as a draft. It read any post in `content/` as published.

So `khalid publish` on an undated post in `content/` changed nothing and still printed "Published". `khalid new` could not make a draft at all, and no message named `drafts/`.

## Decision

- A post is live when its file is in `content/` and has a date. Anything else is a draft: a file in `drafts/<language>/`, or an undated post in `content/`.
- `drafts/<language>/<slug>.md` is where drafts are made. `khalid new --draft`, the Admin API, SerpDad and every importer write there. Messages, help and docs name that folder.
- Publishing a draft in `drafts/` works as ADR 0030 says. Publishing an undated post in `content/` dates it today behind the same lint gate and keeps the file, and any pictures beside it, where it is.
- The post store reports an undated post in `content/` as a draft, in reads, lists and the publish door alike.
- `khalid publish` refuses a post that is already live, with exit code 1 and a message that says how to take it down. It prints "Published" only when the door answers with a live post.

## Consequences

- One rule covers every reader of the files, so a command can never report a change it did not make.
- The build's refusal now says to move a post to `drafts/<language>/` rather than remove its date. Removing the date still works, and is no longer the advice.
- Renaming an undated post's slug through the Admin API moves it into `drafts/`, as for any draft.
