# Changelog

What changed for people who use Khalid, one line per change, newest first. Grouped by Added, Changed, Fixed and Removed. Every pull request that changes behaviour adds its line under Unreleased (`docs/standards/git.md`).

## Unreleased

### Added

- `khalid --site <folder> db migrate` makes the site's database at `data/site.db`, or brings it up to date.
- `khalid --help` lists every planned command: init, serve, new, lint, publish, theme, import, export, mcp, doctor, backup, audit, update, pull and keys.
- Each command explains in three plain sentences that it is not built yet, and exits with code 3.
- `--json` on every command prints the same message as JSON, for scripts and agents.
- A mistyped command gets a plain explanation and exit code 2, instead of a wall of usage text.
- Release builds for Linux (x86_64 and ARM), macOS and Windows, with checksums, from a version tag.
