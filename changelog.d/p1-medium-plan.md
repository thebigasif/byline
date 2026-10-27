### Added

- `crates/khalid-import` plans the files of a Medium import and writes them atomically: published posts under `content/<lang>/posts/` with their date, drafts under `drafts/<lang>/`, the subtitle as the description, and each post's Medium slug kept without its id. Two posts with one slug stay apart, and every file is read back before it is planned (KH-API-7).
