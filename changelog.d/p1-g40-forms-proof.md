### Added

- Form answers now reach you. When a reader sends a form block on `khalid serve`, Khalid emails the answer to `forms_to` under `[mail]` in `site.toml`, or to the `from` address when that is not set, and fires the `form.submitted` webhook with every field. Both are under way within 5 seconds, and the reader's thank-you page never waits for them (G40, KH-GRO-1).
