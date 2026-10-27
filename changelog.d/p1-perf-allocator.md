Changed
`scripts/perf.sh` now measures idle memory the way the CTO defined it: the memory `khalid serve` holds 15 seconds after its first answer, with no requests in between. `docs/standards/performance.md` records which allocator settings were tried and why none is changed (BL-PERF-1, KH-OPS-4).
