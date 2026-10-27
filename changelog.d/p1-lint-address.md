### Fixed

- `khalid lint`, the build's publish gate and the Admin API now read each post at the address its page is built at. A post without `slug:` is named from its title, as `khalid build` names it, not from its file name, and its address follows `routes.toml`. Wikilinks count as the links they become, so a post reached only by `[[slug]]` is no longer an orphan. Two collections whose addresses could clash now get the `overlapping-collections` warning, and a post the home page does not list gets `too-deep` (KH-LNT-2, KH-CON-9).
