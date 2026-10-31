# 0000. Static lock cards: the grey card style, and lock text fixtures

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase 5, static mode (KH-OPS-1), brief G47.

## Context

The ADR "static mode locks forms after the page is rendered" puts a note where each form was: `<p class="locked" data-feature="...">`. The default stylesheet has no rule for `locked`, so the note shows as a plain paragraph and a reader cannot tell it from the post's own text. The plan (section 7) asks for every lock to be a grey card. The stylesheet already has a grey card, `.lock`, which the search page uses for its lock.

KH-OPS-1 is tested by lock text fixtures. Today the notes are checked by string literals in Rust tests, which a reviewer must read as escaped strings.

## Decision

- **The note carries both classes: `<p class="lock locked" data-feature="newsletter">`.** `lock` gives it the grey card the search page already has. `locked` stays, so a theme that styled the note keeps working. No stylesheet changes.
- **Lock text fixtures live in `crates/khalid-static/fixtures/locks/`, one file per case.** Each file holds a page, a `<!-- locked -->` line, and the same page as a static site shows it. The file's name starts with the page's language. One test reads them all. A new kind of lock gets a fixture first.
- **Comments and gated content are locked as whole sections.** Locking only their forms leaves a heading, "No comments yet" and "Leave a comment" above the note, or the writer's "Subscribers read the rest" above a note that says signing up is closed. So a `<section>` whose classes include `comments` or `gated` is swapped whole for one note, up to the `</section>` that closes it, counting sections inside it. A form inside goes with it. Gated content gets its own sentence: "The rest of this post is for subscribers, and signing up is not open here." Its feature is `newsletter`, because gated content opens with a sign-up. A section left open stops the build, as a form left open does.
- **Readers keep the one sentence** decided for 5.3. The owner's card (what, why, fix) stays in `khalid build` and `khalid doctor`.

## Consequences

- Every note is now visibly a card, in every theme that keeps the default stylesheet.
- The fixtures show, as plain HTML, exactly what a reader sees in each language.
- A theme that names its own section `comments` or `gated` for something else loses it on a static site. The class names are the ones Khalid's templates and blocks write, so a theme that keeps them keeps the lock.
