### Added

- Fonts a site's licence lets it serve itself are now served by the site: `theme sync` downloads each rehostable font into `theme/fonts/`, the tokens name the files, the stylesheet links them with `font-display: swap` and each subset's `unicode-range`, and `khalid build` serves them under `assets/fonts/` (G20). A font that cannot be read is skipped, never fatal; the blog keeps working with the next font in the stack.
