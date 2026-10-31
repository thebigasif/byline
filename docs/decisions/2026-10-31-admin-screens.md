# The admin screens of brief G49, 2026-10-31

Brief G49 in `docs/audits/2026-10-27-gaps-2.md` asks for the founder's yes on each admin mockup, a settled mockup folder, and then one screen per part. The founder said yes in advance to everything in the plan on 2026-10-27 (`docs/process/build-pipeline.md`). This file records what that yes covers, so no screen waits.

## 1. The yes covers these mockups

All are in `docs/design/admin-mockups/`. Each opens offline in a browser.

| Mockup | Screen | Drawn |
|---|---|---|
| `dashboard.html` | Dashboard | 2026-10-26 |
| `posts.html` | Posts | 2026-10-26 |
| `editor.html` | Editor | 2026-10-26, preview link and publish time added 2026-10-27 |
| `design.html` | Design | 2026-10-26 |
| `redirects.html` | Redirects | 2026-10-27 |
| `api-keys.html` | API keys | 2026-10-26 |
| `settings.html` | Settings | 2026-10-26 |
| `sign-in.html` | Sign in and start a post | 2026-10-26 |
| `authors.html` | Authors | 2026-10-31, this brief |
| `comments.html` | Comments | 2026-10-31, this brief |
| `account.html` | Your account: passkeys, password, sign out everywhere | 2026-10-31, this brief |

The founder can still say no to any of them. A no reopens that screen; the screens already built stay until its new mockup has a yes.

## 2. One mockup folder

Mockups live in `docs/design/admin-mockups/`, where all eleven already are. `CLAUDE.md`, the `khalid-way` skill and `docs/standards/typescript.md` named `docs/mockups/`, a folder that never existed. `CLAUDE.md` and `typescript.md` now name the real one. Builders may not edit `.claude/skills/`, so the CTO changes line 40 of `khalid-way/SKILL.md` the same way at merge.

## 3. Where a screen differs from its mockup

The screens follow what the Admin API can do today. A mockup part with no route is left out, and the screen says nothing about it rather than show a control that cannot work.

- **API keys.** The Admin API makes content and integration keys only. Staff tokens belong to staff accounts and are made by signing in, so the screen lists staff tokens but does not make them. The API records no "last used" time, so that column shows when the key was made.
- **Settings.** The API changes the name, address, folder and languages. How visitors get the pages (served, plain files or headless) is chosen when Khalid starts, so the screen leaves it out.
- **Design.** No route reads or writes `theme/tokens.toml`, so the screen waits (task 4.7 in `docs/phases/phase-4.md`).
- **Account.** The API adds passkeys but cannot list or remove them yet, so the screen adds one and says that deleting a passkey from the device that keeps it stops it working.

## 4. Screens on routes still in review

The author, key and settings routes are in the G24 stack, open as pull requests on 2026-10-31. These screens call those routes with types written by hand, field for field as G24's OpenAPI document gives them, as the Redirects screen did before its routes landed. On a server without the routes, a screen says that this version of Khalid cannot do it yet and to update.
