# The Content API's store is built behind the pages, without bodies

**Number:** next free.

**Status:** Accepted, 2026-10-27. Phase 1 task 1.17, answering `docs/decisions/2026-10-27-cto-answers.md`, section 3.

## Context

Since #273, `khalid serve` fed the Content API from inside every render of the pages: it read every file again, rebuilt `data/site.db` in one transaction, read every post back, rendered every body a second time and matched each row to its file with a scan over all posts. The pages waited for all of it. On the 10,000-post site the first page took 13.4 s and a saved post 11.5 s to show. The CTO's answer asks that serve answers pages before the API's store is ready, builds that store from the index without loading every post body, and on a save updates only the changed post, with no change to the budgets.

## Decision

- **A render hands the API a catalog and does not wait.** The catalog is the content index (`ContentIndex`), the paths of featured posts and the site's lists, made from the content the render already read. It holds no post body and takes 26 ms for 10,000 posts. A background task (`content_feed.rs`) takes the latest catalog, builds the store, then writes the index. When saves arrive faster than it builds, only the last catalog is built.
- **The store holds no body.** Each published post keeps its details from the index and the path of its file. A request renders the bodies of the posts it returns, on a blocking thread, from their files. This renders at request time, which BL-PERF-2 forbids for pages; it is accepted for the API because an API answer is cached by its ETag, a list returns one page of posts, and every page readers see is still rendered at publish.
- **Requests wait for the first store.** Until the first store is ready, an API request waits instead of answering an empty list. If the first store cannot be built, requests get a 500 problem until the next save. A later failure keeps the last good store, as the pages do.
- **A save updates only what changed.** The task keeps the content hash of every post it last served and wrote. Each new catalog is compared with them: posts whose hash changed are made again, posts whose file is gone are removed, and every other post is kept as it was, in the store and through `Database::update` in `data/site.db`. The small lists (authors, categories, tags) are written whole. After any failure that side is built whole the next time.
- **The index no longer stops serve from starting.** When `data/site.db` cannot be written, for example because two posts share a translation key, serve logs what, why and fix and keeps serving. The database is derived data (ADR 0001) and the API reads nothing back from it.

## Consequences

- On the 10,000-post site, the API adds no time to a saved post: what is left is rendering every page again, which belongs to the server and content tracks. The numbers are in the pull request that lands this ADR.
- A body read through the API comes from the file as it is on disk, which can be a moment newer than the details in the store while a save is being built.
- A store and an index that are built in the background can lag a save by the time a build takes. The end-to-end tests poll for an edit, as they already did for pages.
