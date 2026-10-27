### Added

- `khalid build` on a static site (`mode = "static"` in `site.toml`) lists the six features a static site has off: comments, newsletter sign-up, forms, search, the admin and scheduled publish. Each comes with why it needs a server and how to turn it on, and `--json` gives the same list under `off`. A new `khalid-static` crate names these features (KH-OPS-1).
