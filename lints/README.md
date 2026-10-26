# Lints

`lints.toml` is the lint table, mirrored byte for byte into `Cargo.toml` at the root and into `check/Cargo.toml`. `../clippy.toml` holds the thresholds and the disallowed methods and types. Both are based on BeautifulRust's files, kept here as `*.beautifulrust.toml` so the differences stay visible.

`check/` is Phase 0 work: a crate with one deliberate violation per lint set to warn, each marked with `// expect: <lint>`, so that `../scripts/check-lints.sh` can prove every lint fires and nothing unexpected fires. `check-reference/` is BeautifulRust's version of that crate, for the pattern. Delete `check-reference/` once `check/` exists.
