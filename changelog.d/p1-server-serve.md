### Added

- `khalid --site <folder> serve` runs the blog. It renders every page into memory at start, answers on `127.0.0.1:8080` or the address given with `--listen`, shows a saved edit within about half a second without a restart, sends a Content Security Policy with a fresh nonce on every response, and stops gracefully on Ctrl+C or a stop signal (BL-SEC-2, BL-RUN-10, KH-OPS-5).
