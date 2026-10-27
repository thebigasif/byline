### Changed

- Each page's Brotli copy in `khalid serve` takes only the bytes it holds, not the larger buffer it was compressed into, which frees about half a megabyte on a 1,000-post site (task 1.21).
