### Added

- `khalid lint --fix` makes the safe repairs in the post files, then checks again. It puts headings back in order, so a heading never skips a level. It repairs only what a rule reported, so a rule turned off in `lint.toml` is left alone. It changes only the lines it repairs, and `--json` lists each repair under `fixed` (KH-LNT-3).
