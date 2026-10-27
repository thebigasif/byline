# 0000. Comments are visitor data shown in the rendered page

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 5, task 5.4 (KH-WEB-2, KH-SEO-3, KH-API-3, BL-SEC-5).

## Context

Plan D-4 is settled: Khalid ships its own comments with moderation (`docs/decisions/2026-10-27-phase-5-answers.md`). A reader gives a name and may give an email that is never shown. Rate limits use a salted hash of the network address, the salt rotates daily and the raw address is never stored. Comments are held for approval by default, threaded one level, moderated through the Admin API, announced with the `comment.added` webhook, and listed as `Comment` entries in the post's JSON-LD. Every public form works with scripts off (KH-WEB-2), and the server answers pages from memory without work at request time (BL-PERF-2).

## Decision

- **A new crate, `khalid-comments`, owns the feature**: what a comment is, reading the form with its honeypot, rate limits, moderation and threads. It knows no SQL and no templates. It defines the store as a trait, and `khalid-db` implements it, the same way the webhook delivery log and the API keys work.
- **Comments name their post by its file**, such as `content/en/posts/why-links-matter.md`, so they survive a rebuild (ADR 0010). The form sends the page's address; the server finds the file from its own page index, so a reader can never name a file.
- **Threads are one level deep.** A reply to a reply is stored under the comment that started the thread. A reply under a comment that is not approved is not shown.
- **The honeypot is a field people never see.** A form that fills it in is answered like a kept comment and nothing is stored, so a bot learns nothing.
- **Rate limits live in memory.** Each address is hashed with SHA-256 and a random salt made each day; only the hash and the times of its last comments are kept, for at most 24 hours. A restart forgets them, which only ever lets a few more comments into the moderation queue.
- **Approved comments are part of the rendered post page.** When a comment is approved or deleted, the server renders that one post page again from the comments it holds in memory, the way a saved post shows (the saved-post ADR). The page and its JSON-LD stay bytes in memory, and a request does no work.
- **Without scripts, the form posts and the server answers 303** back to the post at `#comment-sent`, where a note that shows only as the address's target says the comment waits for review. A form with mistakes gets a short page listing them in the reader's language, with a link back.
- **Moderation uses the `posts:write` scope.** Whoever may change posts may moderate the comments on them. A separate scope can come with the admin Comments screen without breaking keys.
- **Static mode needs nothing new.** The form posts to a `comments` folder, so the static lock already replaces it with its note.

## Consequences

- Approving a comment re-renders one post page, which costs what saving that post costs.
- Comments are not in `content/`, so a site's git history does not hold them; the database backup does.
- The admin Comments screen comes from the mockups track and calls the same Admin API routes.
