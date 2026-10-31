# Editor presence, post locking and the table grid, 2026-10-31

The fourth part of gap brief G50 (`docs/audits/2026-10-27-gaps-2.md`) adds two things to the editor: who else has a post open, with a lock so only one person types at a time, and editing a table as a grid of cells. CLAUDE.md asks for an admin mockup and the founder's yes before an admin screen is built. The founder said yes to admin mockups in advance on 2026-10-27 ("you got my all yes. dont stop any work for my yes."), so the mockup ships with the work and the build goes ahead.

## Mockup

`docs/design/admin-mockups/editor-presence-and-tables.html`, listed in the mockup index.

## Decisions

- **E1. The first to open a post holds its lock.** Everyone else who opens it reads it and sees who holds it. The lock lets go when the holder closes the post, or a minute after their editor stops checking in, such as when the laptop sleeps.
- **E2. Taking over is allowed.** Someone reading can take the lock, after the screen says what happens: the holder can only read from then on, and anything they have not saved yet is not kept. This is how WordPress works and matches plan 5.12, "a lock that releases when they leave".
- **E3. The lock is for the editor.** The Admin API, the command line and agents are not refused by it. They are already kept from overwriting a newer save by the stale-edit check, which answers 409.
- **E4. The table grid writes plain markdown tables.** A table opens as a grid of cells with its headings on top. Saving writes a standard markdown table, so the post file stays readable and other tools keep working. A table the grid cannot read stays a card that is edited as markdown.
