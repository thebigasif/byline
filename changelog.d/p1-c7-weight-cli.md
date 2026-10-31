### Added

- The site audit has a page-weight meter: for every page, the bytes of its HTML and of the stylesheets, scripts, images and other files it loads, each file counted once, with the files it could not weigh counted apart. `khalid audit` names the heaviest page and what it weighs, keeps every page's weight in `data/audit/weights.csv`, heaviest first, and `--json` adds the heaviest page and the file's path (task C7, KH-LNT-5).
