### Added

- A post's `schema` field holds hand-written JSON-LD for a type no block writes, such as an Event, a Course or a Product, and the build adds it to the page's structured data. Broken JSON stops the build with a message that names the file. Examples for the nine types without a block, each with everything Google requires, are in `crates/khalid-seo/tests/fixtures/schema/` (ADR "types without a block come from the post's schema field").
