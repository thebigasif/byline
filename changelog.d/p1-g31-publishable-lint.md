### Added

- `khalid-core` and `khalid-lint` are ready for crates.io at version 0.1.0, under the same AGPL-3.0 licence as Khalid, so SerpDad's backend can depend on the linter (KH-LNT-6). `just publish-check` packages both and builds them from the packages without uploading anything.

### Changed

- Every crate names `khalid-core` and `khalid-lint` through the workspace, as `khalid-core.workspace = true`. A new crate that writes `version = "0.0.0"` for them no longer builds.
