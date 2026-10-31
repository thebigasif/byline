# Emoji, photo and GIF search, and picture editing in the editor, 2026-10-31

The last part of gap brief G50 (`docs/audits/2026-10-27-gaps-2.md`) adds the writing aids plan 4.6 still lists: emoji autocomplete with `:`, Unsplash and GIF search, and basic image editing. CLAUDE.md asks for an admin mockup and the founder's yes before an admin screen is built. The founder said yes to admin mockups in advance on 2026-10-27 ("you got my all yes. dont stop any work for my yes."), so the mockup ships with the work and the build goes ahead.

## Mockup

`docs/design/admin-mockups/editor-emoji-photos-and-edits.html`, listed in the mockup index.

## Decisions

- **E5. An emoji goes into the post as itself.** Typing `:` and two letters of a name, after a space or at the start of a line, opens a list of matching emoji. Picking one writes the emoji character, not a `:code:`, so the post file, feeds and every other tool show it as it is. The list is a built-in set of common emoji, so no new package is needed.
- **E6. Photo and GIF search are off until the owner turns them on.** `[media] unsplash = true` and `[media] gifs = true` in `site.toml` turn each on. The keys come from the environment only, `KHALID_UNSPLASH_ACCESS_KEY` and `KHALID_GIPHY_API_KEY`, so they never sit in a file or a backup. The blog asks Unsplash or GIPHY for the writer, so the key never reaches the browser. A search that is on but has no key says which variable to set.
- **E7. A chosen photo or GIF stays where it came from.** Unsplash's API terms ask apps to show photos from Unsplash's own addresses, to credit the photographer with links, and to tell Unsplash when a photo is used. So the editor adds a figure whose picture loads from Unsplash, with "Photo by <name> on Unsplash" as its caption, and the blog tells Unsplash it was used. A GIF loads from GIPHY with "Via GIPHY" under it. Readers' browsers then fetch that one picture from Unsplash or GIPHY, which is why both are off by default. The public pages already allow pictures from other secure sites; the admin's policy gains the two picture hosts so the search results show.
- **E8. GIFs come from GIPHY.** It has a public search API with free keys. One source keeps the setting and the screen simple.
- **E9. Turning or cropping makes a new copy.** The original stays in the media library, so other posts that use it do not change. The copy keeps the alt text, the caption and the folder, and its focus point moves with the picture. Pictures turn in quarter turns. The editor crops to a shape (as taken, square, wide 16 by 9, standing 3 by 4) around the focus point; the Admin API takes any box.
- **E10. Only library pictures can be turned or cropped.** The blog can only change files it keeps. A photo from Unsplash or a picture at another address shows no "Turn or crop" button.
- **E11. Annotating is left for later.** Drawing on a picture needs a canvas in the browser that cannot be tested on this server; turning and cropping cover most needs.
