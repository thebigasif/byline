# Changelog

What changed for people who use Khalid, one line per change, newest first. Grouped by Added, Changed, Fixed and Removed. Every pull request that changes behaviour adds its line under Unreleased (`docs/standards/git.md`).

## Unreleased

### Added

- `khalid --site <folder> lint` checks every post against the SEO rules and explains each problem in three plain sentences. It exits with 1 when a rule blocks publishing, `--json` prints the findings for scripts, and `lint.toml` turns rules into warnings or off (KH-LNT-1, KH-LNT-2, KH-LNT-3).
- `khalid --site <folder> db migrate` makes the site's database at `data/site.db`, or brings it up to date.
- `khalid --help` lists every planned command: init, serve, new, lint, publish, theme, import, export, mcp, doctor, backup, audit, update, pull and keys.
- Each command explains in three plain sentences that it is not built yet, and exits with code 3.
- `--json` on every command prints the same message as JSON, for scripts and agents.
- A mistyped command gets a plain explanation and exit code 2, instead of a wall of usage text.
- `khalid --site <folder>` checks the site's settings and reads every content file: posts, pages, authors, glossary terms, case studies, changelog entries and snippets, as plain files or as folders with their images. A mistake is explained by file, line and field (KH-CON-1, KH-CON-2, KH-CON-3, KH-CON-4).
- Release builds for Linux (x86_64 and ARM), macOS and Windows, with checksums, from a version tag.
