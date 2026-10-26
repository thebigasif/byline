Changed
`khalid serve` reads the site's posts once at start instead of twice, and no longer keeps the copy it checked, so it starts sooner and holds about 20 MB less memory on a 1,000-post site (BL-PERF-1, KH-OPS-4).
