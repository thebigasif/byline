### Added

- `khalid theme sync <url>` reads your main site's colours and fonts and saves them in `theme/tokens.toml`, telling you where the file is and which colours it made easier to read. When the site cannot be reached, has no stylesheet, or points to a private address, it says what happened and what to do, and changes nothing. `--json` gives the same result for scripts (KH-OPS-2, KH-WEB-8).
