# 0000. Form answers are emailed once, kept always, and sent to the webhooks with retries

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase 1, G40 form delivery (KH-GRO-1).

## Context

G40 asks that each form answer reach email and the `form.submitted` webhook within 5 seconds. The form block names no destination, `khalid-content` belongs to the content track, and the plan does not say where the email goes or what happens when sending fails.

## Decision

- The address is `forms_to` under `[mail]` in `site.toml`, and `from` when it is not set. A site that sends mail delivers its forms with no further setting.
- `khalid-forms` gains a `FormDelivery` trait. The route calls it after an answer is kept, never for bait or an answer that could not be written. The binary's `SiteDelivery` runs both sends on the announcing tasks, which the server waits for as it stops.
- The webhook goes through the existing dispatcher, so a failed delivery is logged and retried like every other event.
- The email is sent once. If the mail server refuses it, the failure is logged with the form id and no field values, and the answer stays in `form_submissions`. Mail has no delivery log yet, and a retry queue for mail belongs with the newsletter's sends (G39).
- The webhook body keeps every field, the reader's email included, because answering the reader is the point of a form. `comment.added` hides it because a comment's email is never shown.

## Consequences

- `crates/khalid/tests/serve_form_delivery.rs` proves both arrive within 5 seconds, and that with the mail server down the reader is still thanked and the webhook still fires.
- An owner whose mail server is down for a while misses those emails, but finds the answers in the database, and later in the admin's Forms screen (G49).
- The `form-without-destination` lint rule can count a site with `[mail]`, or with a `form.submitted` webhook, as a destination when G5 fills `LintForm::destinations`.
