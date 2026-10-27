### Changed

- `khalid-import` keeps what every importer shares at the crate root: `PlannedFile`, `WriteError`, `SlugOrigin`, `MapTarget`, `Matching` and `MoveStatus`, the redirect CSV and the safe writer, so the Ghost importer uses the same ones as the WordPress importer (KH-API-7).
