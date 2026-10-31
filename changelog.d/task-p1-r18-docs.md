### Added

- `docs/getting-started.md` takes a new user from a new site to a published post, then through the admin sign-in, lint, build, audit, imports, and backup and restore. A test runs every command on the page.
- `CONTRIBUTING.md` says how to build Khalid from source, admin included, and that `cargo` alone builds it without the admin.

### Changed

- The README's install section works before the first release: it builds from source, and the one-line installer is shown for later.
- `docs/deploy.md` names `khalid backup` and `khalid restore` and shows how to put a backup in place of a live site.
