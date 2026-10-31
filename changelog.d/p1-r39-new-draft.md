### Added

- `khalid new --draft "Title"` writes an undated draft in `drafts/<language>/<slug>.md`, which readers never see, and prints the `khalid publish` command that takes it live. Without `--draft`, `new` says the post goes live at the next build and where drafts go instead (KH-EDT-4, KH-OPS-3).
