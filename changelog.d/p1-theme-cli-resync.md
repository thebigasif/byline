### Added

- Running `khalid theme sync` again keeps every value you listed in `locked` in `theme/tokens.toml`, takes the main site's new values for the rest, and prints one line for each value that changed. If the file has a mistake, such as a missing value, it names the line and changes nothing (KH-OPS-2, KH-WEB-8).
