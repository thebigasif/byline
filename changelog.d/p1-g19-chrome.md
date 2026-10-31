### Added

- Every page shows the menus written in `site.toml`: `[[nav.primary]]` in the header and `[[nav.footer]]` in the footer, in the order written, each link with a `language` only on that language's pages. A link to the page shown is marked as the current page (G19).
- An `[announcement]` in `site.toml` shows one line, linked or not, in a band above the header of every page, or of one language's pages. It is plain HTML, so it reads the same with scripts off, and it is left out of print (G19).
