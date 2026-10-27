### Changed

- `khalid serve` keeps one copy of each page in memory, its compressed copy, instead of two. A reader whose browser does not take Brotli gets the page decompressed as it is sent. On a 1,000-post site the server holds about 15 MB less (KH-OPS-4).
