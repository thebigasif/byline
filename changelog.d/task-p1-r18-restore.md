### Added

- `khalid restore <archive> <folder>` makes a site again from a `khalid backup` archive, database included. It only fills a new or empty folder, so a live site is never unpacked over, and it refuses an archive that is not a site backup or names a file outside its folder.
