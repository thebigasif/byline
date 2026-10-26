#!/usr/bin/env bash
# Runs every budget measure that can run on a CI runner, writes
# perf/current.json, and compares it with perf/baseline.json through
# scripts/perf-gate.py. See docs/standards/performance.md.
#
# Measured today: the release binary's size and the cold start of
# `byline --help`. Later phases add the rest of the budgets here.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

cargo build --release -p byline
bin="${CARGO_TARGET_DIR:-$root/target}/release/byline"

python3 - "$bin" perf/current.json <<'PY'
import json
import os
import statistics
import subprocess
import sys
import time

binary, out = sys.argv[1:]
runs = []
for attempt in range(23):
    start = time.perf_counter()
    subprocess.run([binary, "--help"], check=True, stdout=subprocess.DEVNULL)
    if attempt >= 3:  # the first three warm the file cache
        runs.append((time.perf_counter() - start) * 1000)
measures = {
    "binary_size_bytes": os.path.getsize(binary),
    "cold_start_help_ms": round(statistics.median(runs), 2),
}
with open(out, "w") as f:
    json.dump({"measures": measures}, f, indent=2)
    f.write("\n")
PY

python3 scripts/perf-gate.py perf/baseline.json perf/current.json
