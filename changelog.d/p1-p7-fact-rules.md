### Added

- Two lint warnings for the fact registry: `fact-old-value` when a post still gives a value a fact held before, with the current value as the fix, and `fact-not-checked` when a post gives a fact nobody has checked for longer than `fact-check-months` in `lint.toml`, six months by default.
