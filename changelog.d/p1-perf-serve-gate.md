Added
`scripts/perf.sh` now times `khalid serve` on the generated 1,000-post site: how long a start takes to send the first byte, how much memory the server holds when idle, and how long a saved post takes to show. Each is checked against its budget in `perf/baseline.json`. The same three on the 10,000-post site are reported beside the check (BL-PERF-1, KH-OPS-4).
