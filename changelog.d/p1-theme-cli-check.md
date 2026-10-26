### Added

- `khalid theme check` compares `theme/tokens.toml` with your main site and with WCAG AA, the web's standard for readable pages. It lists each value that no longer matches the site and each colour too faint to read, changes nothing, and exits with 1 when something needs you, so a script or a scheduled job can watch for it. It reads the main site's address from `site.toml` unless you give one, and `--json` gives the same result for scripts (KH-OPS-2, KH-WEB-8).
