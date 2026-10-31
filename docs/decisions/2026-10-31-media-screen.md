# The admin Media screen, 2026-10-31

Brief G51 in `docs/audits/2026-10-27-gaps-2.md` asks for the media library. On `main` the library itself, its four Admin API routes (upload, list, read, edit) and its serving are done. The missing part is the admin screen.

## The founder's yes

CLAUDE.md says an admin screen needs a mockup and the founder's yes before it is built. On 2026-10-27 the founder said yes in advance to every admin mockup the tracks draw. That standing yes covers the two Media mockups:

- `docs/design/admin-mockups/media.html`: upload a file into a folder, and the library, newest first, with a switch to show only images that have no alt text.
- `docs/design/admin-mockups/media-details.html`: one file's alt text, caption, folder and focal point, and its address to copy.

The screen is built from these mockups in the same pull request. If the founder later asks for a change, it is a change to a built screen, not a blocker.

## Choices taken

- Media sits in the side menu after Posts, because writers reach for it while writing.
- The focal point is set by clicking the picture, or by typing two percentages for keyboard and screen reader users. The API keeps thousandths; the screen shows whole percent.
- Deleting a file is not on the screen. The API has no delete route yet, and deleting a file a post still uses would break that post.
- Folders are typed, not picked from a tree. The API has no folder list, and a typed folder is created when the first file lands in it.
