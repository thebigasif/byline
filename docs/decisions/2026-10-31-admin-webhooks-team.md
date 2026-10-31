# The Webhooks and Team admin screens, 2026-10-31

Brief G49 in `docs/audits/2026-10-27-gaps-2.md` asks for the admin screens of plan section 4.6. The first G49 run built Comments, Account, Authors, API keys and Settings (tasks 4.1 to 4.6 in `docs/phases/phase-4.md`). This run was asked for Comments, Authors and API keys again; they already exist, so it builds the next screens whose Admin API routes are on `main`: Webhooks and Team.

## 1. Why these two

Every other screen still missing from section 4.6 (Calendar, Collections, Snippets, Forms, Subscribers, Navigation, Lint, Audit, Search Console, Sites, Design) has no Admin API route on `main`. Webhooks (`/api/admin/v1/webhooks/`, task 1.19) and staff accounts (`/api/admin/v1/staff/`, brief G48) do, and no screen calls them.

## 2. The yes covers these mockups

The founder said yes in advance to everything in the plan on 2026-10-27. That yes covers the two new mockups in `docs/design/admin-mockups/`, the same way as the eleven in `docs/decisions/2026-10-31-admin-screens.md`:

| Mockup | Screen | Drawn |
|---|---|---|
| `webhooks.html` | Webhooks | 2026-10-31 |
| `team.html` | Team: invite, change a role, remove | 2026-10-31 |

The older mockups keep their menus; the two new ones show where the screens sit in it.

## 3. Where a screen differs from the plan or its mockup

- **No delivery log.** Plan section 5.12 wants a delivery log in the admin. The API keeps the log in `webhook_deliveries` but has no route that reads it, so the screen leaves it out and says nothing about it.
- **Events as sentences.** Each event shows as a short sentence, such as "A post goes live", with its API name beside it in the list. An event the admin does not know yet shows by its API name, so a newer server never breaks the screen.
- **No email on invite.** The staff route makes the account and sends nothing. The screen says so, and tells the person inviting to send the new person to the sign-in screen, where an email sign-in mails them a link.
- **No owner hand-over.** `POST /staff/{id}/owner/` hands the whole site to someone else. It is rare, cannot be undone by the person who did it, and needs its own confirmation screen, so this run leaves it out. `khalid staff` covers it until then.
- **Every control shows, the API decides.** The screen does not ask who is signed in before it shows Change role and Remove. The API refuses what a role may not do with a 403 in plain words, and the screen shows those words. The owner's row has no controls, because nobody may change the owner.
- **Team, not Staff.** The menu says Team, the plain word for the people who sign in. The API keeps the name `staff`.
