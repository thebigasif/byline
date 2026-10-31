### Added

- `khalid export` writes a static site into `public/` whole: it builds into a new folder and swaps it in only once every page is written, so a host never copies half a site and pages of deleted posts are gone. A refused build leaves the earlier export in place. The notes for a static site now point to `khalid export` (KH-OPS-1, KH-OPS-2).
