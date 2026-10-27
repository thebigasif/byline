### Added

- The linter warns when the focus keyword appears too rarely or too often in a post of 100 words or more: under 5 or over 25 times per thousand words, Rank Math's band of half a percent to two and a half, tunable as `focus-min-per-thousand-words` and `focus-max-per-thousand-words` in `lint.toml` (KH-LNT-4).
- The linter warns about each secondary keyword the text never uses.

### Changed

- `keyword-stuffing` leaves the focus keyword to `focus-too-often`, so a writer gets one warning for one problem.
