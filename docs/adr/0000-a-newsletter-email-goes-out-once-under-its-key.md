# 0000. A newsletter email goes out once, under its key

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase 5 tasks 5.7 to 5.10 and 5.12, brief G39 in `docs/audits/2026-10-27-gaps-2.md`, KH-GRO-2, KH-GRO-3.

## Context

Task 5.5 built sign-ups and 5.6 the transport. The founder's answers of 2026-10-27 (newsletter 2, 3, 4 and 7) settle what comes next: several newsletters named by `list=` on the block, welcome emails in `content/newsletters/<name>/welcome.md`, a post sent only when its writer asks and only once, no sending until the site has a postal address, and numbers as totals per email with open counting off. The plan adds segments and labels, an embeddable form and RSS to email. There is no Admin API for subscribers yet, and the Subscribers screen comes from the mockups track.

## Decision

- **A newsletter is a folder.** `main` always exists; every folder in `content/newsletters/` with a slug for a name is one more. A form can only join one of these, so readers cannot invent lists. The content walker skips the folder, so nothing in it becomes a page.
- **Every send has a key, unique per site.** `newsletter_sends` (migration 0020) holds one row per email: `post:<language>/<slug>` for a post, `digest:<list>:<date of the newest post>` for a digest. The row is written before the first copy goes out, so an edit, a second run or a retry after a failure never sends anyone a copy twice. A copy the transport refuses is counted, not retried.
- **Sending takes a `PostalAddress`.** `Newsletter::send` and `Welcomes` cannot be built without one, so "refuse until the site has a postal address" is a type, not a check someone can forget. It lives in `[mail] postal_address`.
- **Every copy carries its own unsubscribe link.** The token remembers the send it came in (`subscriber_tokens.send_id`), so a first click counts once against that email's `unsubscribed` total. Who clicked is not stored anywhere else.
- **Numbers are totals.** Sent, refused and unsubscribed per email. Opens are not counted: that needs a tracking image per reader. Click counting needs a redirect route that rewrites links and is left for later.
- **Labels make segments.** `subscriber_labels` (migration 0019) holds slugs per subscriber; a segment is one newsletter's confirmed subscribers who carry every label named. Commands print counts, never people.
- **The command line is the writer's ask.** `khalid newsletter send`, `digest`, `label`, `segment`, `numbers` and `embed` call `khalid-newsletter` and the `SubscriberTable` directly, as `khalid webhooks` does, until Admin API routes for subscribers exist. The admin's "send as email" tick comes with the mockups track and calls the same `Newsletter::send`.
- **A welcome email is a markdown file whose first line is a `# ` heading**, the subject. No front matter. It goes out when a confirmation link confirms a reader now, once per newsletter they joined.
- **The embeddable form is HTML to paste, not a frame.** The newsletter pages keep `frame-ancestors 'none'`. The snippet posts to the blog's full address and works with scripts off.
- **RSS to email is a command for a timer**, not a background job in `khalid serve`: the digest lists the default language's posts dated after the last digest's newest post, or from the last seven days the first time.

## Consequences

- A post dated the same day as the newest post of the last digest, but published after it went out, is not in the next digest. Running the digest once a day or less keeps this rare.
- A confirmed reader who joins a second newsletter gets no welcome for it, since the store does not say the join is new.
- `khalid serve` reads the welcome files at start; an edited welcome needs a restart.
