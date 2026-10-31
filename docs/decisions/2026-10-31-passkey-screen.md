# The admin Passkeys screen, 2026-10-31

Task S1 (`docs/phases/phase-geo.md`) moved staff passkeys onto `webauthn-rs` and left one gap: no admin screen to add, name and remove them. This run fills it. It branches from `task/p1-integration-15`, the open integration that carries S1, because `main` does not have S1's routes yet.

## The founder's yes

CLAUDE.md says an admin screen needs a mockup and the founder's yes before it is built. On 2026-10-27 the founder said yes in advance to every admin mockup the tracks draw. That standing yes covers `docs/design/admin-mockups/passkeys.html`: what a passkey is, adding one on this device, and the list of your own passkeys with renaming and removing.

The screen is built from this mockup in the same stack. If the founder later asks for a change, it is a change to a built screen, not a blocker.

## Choices taken

- **Its own screen, in the side menu.** Passkeys sit at `/admin/passkeys`, after API keys. The G49 stack (`task/p1-g49-*`, not yet in an integration) has a "Your account" screen whose add form was written for the passkey routes before S1. S1 changed what those routes take, so that form no longer fits. When both land, the account screen should drop its form and link here.
- **A passkey belongs to a person, not a key.** The list, rename and remove routes read the session cookie, as adding one already did. A tab signed in with a staff token sees one sentence saying to sign in with an email instead. The email sign-in comes with G49.
- **Names are 1 to 60 characters, trimmed.** Adding a passkey now checks its name the same way.
- **Removing the last passkey is allowed.** Every account keeps its password and email links, so nobody is locked out. The screen says so before it asks.
- **Another person's passkey answers 404**, the same as one that does not exist, so the answer teaches nothing.
