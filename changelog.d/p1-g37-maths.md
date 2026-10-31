### Added

- Posts can show a formula on its own with a `:::maths` block, with an optional `label` as its caption. It is drawn as MathML when the site builds, so readers see it with scripts off, and a formula that cannot be drawn is refused with its file and line (KH-CON-5, KH-WEB-9).

### Fixed

- The linter no longer reads the letters and signs of a formula as words, so maths cannot trip the writing checks.
