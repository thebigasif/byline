# Phase 4: the admin screens

Brief G49 in `docs/audits/2026-10-27-gaps-2.md`, written 2026-10-31. Plan: `docs/plan.md` section 4.6 and Phase 4. Decision: `docs/decisions/2026-10-31-admin-screens.md`. Each screen follows its mockup in `docs/design/admin-mockups/`, calls only Admin API routes, and is tested with Vitest, Testing Library and axe in CI's Admin job.

Other briefs own the rest of section 4.6: the editor (G50), Media (G51), the toolbar, audit log, trash and bulk actions (G52), review rules and the calendar (G53), SEO in the admin (G54) and accounts (G48). Collections, Snippets, Forms, Subscribers, Navigation, Lint, Audit, Search Console and Sites get their own brief once their routes exist.

## Tasks

### 4.1 Mockups and the founder's yes
- Status: `todo`.
- New mockups for Authors, Comments and Account (passkeys and password) beside the first eight, in the same style. The founder's standing yes of 2026-10-27 covers them, recorded in the decision above.
- Done when: `docs/design/admin-mockups/index.html` links every screen this phase builds, and the decision names each one.

### 4.2 Comments
- Status: `todo`.
- The moderation queue on `GET /api/admin/v1/comments/`: waiting comments first, each with Approve, Reply and Delete. The other statuses one switch away.
- Done when: a test approves one comment, replies to another, deletes a third, and axe finds nothing.

### 4.3 Account and passkeys
- Status: `todo`.
- Add a passkey through `navigator.credentials.create` and the two `session/passkeys` routes, change the password, and sign out everywhere.
- Done when: a test adds a passkey with a stubbed browser credential and checks the base64url body the API gets, a wrong password shows the API's words, and axe finds nothing.

### 4.4 Authors
- Status: `todo`.
- List, add, edit and delete authors on the G24 author routes, with the `updated_at` each edit carries.
- Done when: tests add an author, edit one, delete one, show a 409 in plain words, and axe finds nothing.

### 4.5 API keys
- Status: `todo`.
- Make a content or integration key, see it once, and switch keys off, on the G24 key routes.
- Done when: a test makes a key, sees it once and never again after a reload, switches one off, and the API's 409 for the key in use reads in plain words.

### 4.6 Settings
- Status: `todo`.
- Name, address, folder and languages on the G24 settings routes, with the warning beside the address and a note when a restart is needed.
- Done when: a test saves a change, sends only the changed fields with `updated_at`, and shows the restart note.

### 4.7 Design
- Status: `blocked`.
- The mockup is approved. No route reads or writes `theme/tokens.toml` yet, on `main` or any open branch.
- Done when: the screen edits colours and fonts through an Admin API route, and a test shows the contrast score beside each colour.

### 4.8 Webhooks
- Status: `todo`.
- List, add and switch off webhooks on `/api/admin/v1/webhooks/`, each event in a plain sentence, the signing secret shown once. Decision: `docs/decisions/2026-10-31-admin-webhooks-team.md`.
- Done when: a test adds a webhook and sees its secret once, switches one off, shows the API's 422 for a bad address in plain words, and axe finds nothing.

### 4.9 Team
- Status: `todo`.
- List staff, invite someone with a role, change a role and remove someone on `/api/admin/v1/staff/`. Handing the site to a new owner waits for its own screen.
- Done when: a test invites someone, changes a role, removes someone, shows the API's 403 and 409 in plain words, and axe finds nothing.

### 4.10 Webhook activity
- Status: `todo`.
- Show what one webhook sent, newest first, on a new `GET /api/admin/v1/webhooks/{id}/deliveries/`: delivered, trying again or given up, with each try. Decision: `docs/decisions/2026-10-31-admin-activity-owner.md`.
- Done when: a request test lists a webhook's deliveries with their attempts from SQLite, a screen test shows each state in words, and axe finds nothing.

### 4.11 Hand the site on
- Status: `todo`.
- Make another person on the team the owner with `POST /api/admin/v1/staff/{id}/owner/`, on its own screen that says what changes before it asks.
- Done when: a test hands the site to someone and sees them as owner, shows the API's 403 in plain words, and axe finds nothing.

## Questions for the founder

None. Choices taken without asking are in the decision above.
