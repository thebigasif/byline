# The webhook activity and owner hand-over admin screens, 2026-10-31

Brief G49 in `docs/audits/2026-10-27-gaps-2.md` asks for the admin screens of plan section 4.6. The first two G49 runs built Comments, Account, Authors, API keys, Settings, Webhooks and Team (tasks 4.1 to 4.9 in `docs/phases/phase-4.md`). Their decision, `docs/decisions/2026-10-31-admin-webhooks-team.md`, left two gaps: the webhook delivery log and handing the site to a new owner. This run closes both. It builds on the top of the G49b stack, `task/p1-g49b-team`, because both screens extend Webhooks and Team.

## 1. Which activity log

This run was asked for "the activity log". The plan has two logs an admin reads:

- **Who changed what** (plan 5.12, "Audit log"). Brief G52 built it: the `GET /api/admin/v1/audit/` route and an Audit log screen, in pull request #880, waiting in integration pull request #892. Building it again would collide with that work.
- **What each webhook sent** (plan 5.12, "A delivery log in the admin"). The log is kept in `webhook_deliveries` since task 1.19, but no route reads it and no screen shows it.

So the activity log here is the webhook delivery log. The screen is called "What this webhook sent", reached from "See what it sent" beside each webhook.

## 2. The yes covers these mockups

The founder said yes in advance to everything in the plan on 2026-10-27. That yes covers the two new mockups in `docs/design/admin-mockups/`, the same way as the thirteen before them:

| Mockup | Screen | Drawn |
|---|---|---|
| `webhook-activity.html` | What this webhook sent | 2026-10-31 |
| `owner.html` | Hand the site to someone else | 2026-10-31 |

`webhooks.html` and `team.html` gain the links that open them.

## 3. Choices taken

- **A new route for the log.** `GET /api/admin/v1/webhooks/{id}/deliveries/` lists the 50 newest deliveries of one webhook that is on, newest first, each with every attempt. It needs `webhooks:manage`, like the other webhook routes. There is no paging: a webhook that fires more than 50 times between looks has its older messages in `khalid db export`.
- **No message bodies.** The route leaves out the bytes sent. They can be large, and the event and time say what was sent. A switched-off webhook's log is not listed, because no screen lists switched-off webhooks; its rows stay in the database.
- **Every try shows.** A delivery has at most seven tries, so the screen lists them all rather than folding them.
- **The hand-over is its own screen.** It says what changes and that it cannot be undone by the person doing it, before it asks for anything. A checkbox confirms that, and the button names the new owner. The screen does not check who is signed in: the API answers 403 for anyone but the owner, and the screen shows those words.
