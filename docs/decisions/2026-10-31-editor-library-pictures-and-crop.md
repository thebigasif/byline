# Library pictures, a crop box to drag, and picture search in doctor and Settings, 2026-10-31

The follow-ups gap brief G50's last part left open (`docs/decisions/2026-10-31-editor-emoji-photos-and-edits.md`, decisions E5 to E11): inserting any library picture from the editor, a crop box the writer drags, and showing photo and GIF search in `khalid doctor` and Settings. The founder said yes to admin mockups in advance on 2026-10-27, so the mockup ships with the work and the build goes ahead.

## Mockup

`docs/design/admin-mockups/editor-library-pictures-and-crop.html`, listed in the mockup index.

## Decisions

- **E12. "Picture from your library" is always in the block menu.** Every blog has a media library, so the item needs no setting. The list shows images only, newest upload first, 24 at a time with "Show more pictures". Typing narrows it to pictures whose name or alt text holds the words, through the list route's existing `filter`. A picked picture becomes a figure with the alt text and caption saved in the library; the writer can change both in the post.
- **E13. "Your own box" is a fifth shape.** The writer drags the box to move it and a round handle at its lower right corner to change its size. Four number fields, in whole percent of the picture, do the same without dragging, as WCAG 2.2 asks for anything that needs a drag (2.5.7). The box is at least 5% on each side. Turning starts the box again, because the sides swap. The picture and the box are drawn as one SVG, so the box sits on the turned picture without inline styles.
- **E14. `khalid doctor` gains a "Picture search" check.** It reads `[media]` and the two environment variables. A search that is on without its key is a problem, so doctor exits with 1: the owner asked for it and writers would see a message instead of pictures. Doctor does not send the key to Unsplash or GIPHY, because a check that spends the owner's hourly allowance and needs the network belongs in a later `--online` pass. When `site.toml` does not load, the check is skipped.
- **E15. Settings shows each search as off, ready, or missing its key, and changes nothing.** `GET /api/admin/v1/media/search/` gains `keys`, with `unsplash` and `gifs` true when the server holds that search's key. The section only shows what is set and how to change it, because the keys live in the server's environment and never pass through the browser (E6). The admin's Settings screen is on the G49 stack, which this one does not stack on; the section is its own component, and the integration that joins both adds one line under the site form.
