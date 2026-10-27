### Added

- The Ghost importer carries image, code, markdown, HTML and divider cards: an image becomes a `:::figure` with its caption, code a fenced block with its language and every space kept, and markdown and HTML cards are cleaned like the rest of the body, so they cannot carry a script or an unsafe link (KH-API-7, BL-SEC-1).

### Fixed

- An imported link or image with an empty address keeps only its words, instead of becoming a link that leads nowhere.
