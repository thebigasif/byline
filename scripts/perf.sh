#!/usr/bin/env bash
# Runs every budget measure that can run on a CI runner, writes
# perf/current.json, and compares it with perf/baseline.json through
# scripts/perf-gate.py. See docs/standards/performance.md.
#
# Measured today: the release binary's size, the cold start of
# `khalid --help`, post page render p50 and p99 over the generated
# 1,000-post site, and a full `khalid build` of the generated 10,000-post
# site. Cold start to first byte, idle memory and incremental rebuild wait
# for `khalid serve`; the lint measures wait for the linter's near-copy
# rule to stop growing with the site (task 1.21).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# Built one at a time, so khalid-perf's dependencies cannot change the
# features, and so the size, of the binary being measured.
cargo build --release -p khalid
cargo build --release -p khalid-perf
release="${CARGO_TARGET_DIR:-$root/target}/release"
bin="$release/khalid"
perf="$release/khalid-perf"

# The generated sites (docs/adr, generated perf fixtures). Never under
# fixtures/ or a content/ folder.
work="$root/target/perf"
rm -rf "$work"
mkdir -p "$work"
"$perf" fixture 1000 "$work/site-1k"
"$perf" fixture 10000 "$work/site-10k"
"$perf" render "$work/site-1k" "$work/render.json"

python3 - "$bin" "$work" perf/current.json <<'PY'
import json
import os
import shutil
import statistics
import subprocess
import sys
import time

binary, work, out = sys.argv[1:]


def timed(command):
    """Seconds one run of `command` takes, which must succeed."""
    start = time.perf_counter()
    subprocess.run(command, check=True, stdout=subprocess.DEVNULL)
    return time.perf_counter() - start


help_runs = []
for attempt in range(23):
    seconds = timed([binary, "--help"])
    if attempt >= 3:  # the first three warm the file cache
        help_runs.append(seconds * 1000)

build_runs = []
target = os.path.join(work, "build-10k")
for attempt in range(3):
    shutil.rmtree(target, ignore_errors=True)  # a full rebuild starts empty
    site = os.path.join(work, "site-10k")
    build_runs.append(timed([binary, "--site", site, "build", "--out", target]))

with open(os.path.join(work, "render.json")) as f:
    render = json.load(f)

measures = {
    "binary_size_bytes": os.path.getsize(binary),
    "cold_start_help_ms": round(statistics.median(help_runs), 2),
    "post_render_p50_ms": render["post_render_p50_ms"],
    "post_render_p99_ms": render["post_render_p99_ms"],
    "full_rebuild_10k_s": round(statistics.median(build_runs), 2),
}
with open(out, "w") as f:
    json.dump({"measures": measures}, f, indent=2)
    f.write("\n")
PY

python3 scripts/perf-gate.py perf/baseline.json perf/current.json
