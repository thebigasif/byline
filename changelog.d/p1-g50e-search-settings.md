### Added

- `site.toml` takes `unsplash = true` and `gifs = true` under `[media]` to turn on photo and GIF search in the editor (G50, KH-EDT-1). Both are off by default. Their keys come from the environment only, as `KHALID_UNSPLASH_ACCESS_KEY` and `KHALID_GIPHY_API_KEY`, and never print.
