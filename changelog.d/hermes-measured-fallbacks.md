### Added

- Every self-hosted font now gets its own tuned local fallback, whatever the family: `theme sync` measures each downloaded file's ascent, descent, line gap and average width, and the stylesheet serves a fallback `@font-face` that holds the same space, so the swap does not shift the page. Until now only Inter and Roboto had one (G20, last part).
