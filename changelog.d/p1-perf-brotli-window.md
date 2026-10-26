Changed
`khalid serve` sizes each page's Brotli compression to the page instead of to 4 MB, so compressing a 1,000-post site holds about 3 MB less memory afterwards, with the same compressed sizes (BL-PERF-1, KH-OPS-4).
