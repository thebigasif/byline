# Lints

`lints.toml` is the lint table. The root `Cargo.toml` mirrors it under `[workspace.lints.*]`, with the same lints, levels and order. `../clippy.toml` holds the thresholds and the disallowed methods and types. Both are based on BeautifulRust's files, kept here as `*.beautifulrust.toml` so the differences stay visible.

`check/` is the proof (RULES.md BL-LNT-3). `check/src/violations.rs` holds one deliberate violation per table entry. Each is marked `// expect: <lint>` for a lint that must fire, or `// quiet: <lint>` for a lint the table allows. The module compiles only when `../scripts/check-lints.sh` sets the `khalid_lint_check` cfg, so normal builds of the workspace stay clean.

`../scripts/check-lints.sh` runs clippy and rustdoc over the violations and fails when:

- the two tables drift apart,
- an entry has no marker, or a marker names no entry,
- a marked lint stays silent, or anything unmarked fires.

It also fails when a method or type disallowed in `../clippy.toml` has no violation in `check/src/violations.rs`.

`../scripts/prove-lint-table.sh` (`just prove-lints`) removes each entry in turn and checks that `check-lints.sh` then fails. It takes a few minutes, so CI does not run it.

To add a lint: add the line to `lints.toml` and `Cargo.toml`, then add a marked violation to `check/src/violations.rs`, then run both scripts.
