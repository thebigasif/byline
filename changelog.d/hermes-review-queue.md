### Added

- A review queue (F3): a `reviewed:` date in a post's front matter records that someone looked it over without touching its `date` or `updated`, `khalid lint` warns when a post has gone six months without a review (decision D4's limit, `review-months` in `lint.toml`), and the Admin API's post rows carry `reviewed` and `review_due` so the dashboard can filter `review:due`.

### Changed

- `khalid serve` answers its first request sooner, with the allocator setting that keeps idle memory in budget. It no longer makes the stylesheet's Brotli and gzip copies before it listens (the server compresses its pages after it answers, as it always did for the stylesheet), it builds the Content API and the Admin API beside the first answer instead of before it, and it starts the full render only once its routes are ready.

### Fixed

- The heading shape check (W6) read list bullets and table pipes from text that had lost them, so it warned on every heading about kinds or differences, list or no list. It now reads the lists and tables under each heading from the rendered page, and only whole words make a promise, so "always" no longer promises "ways".
