#!/usr/bin/env python3
"""Compare perf/current.json with perf/baseline.json (RULES.md BL-PERF-1).

A measure fails when it is worse than its baseline by more than its allowed
drift. A measure in the baseline that perf.sh does not run yet is listed as
not measured. A measure perf.sh runs that the baseline lacks fails, because
every measure needs an accepted number before it can guard anything.

Usage: perf-gate.py BASELINE CURRENT
"""

import json
import sys
from pathlib import Path


def limit(base):
    """The worst value the gate accepts for one measure."""
    if base["better"] == "lower":
        return base["value"] * (1 + base["drift"])
    return base["value"] * (1 - base["drift"])


def main(baseline_path, current_path):
    baseline = json.loads(Path(baseline_path).read_text())["measures"]
    current = json.loads(Path(current_path).read_text())["measures"]
    failed = False
    print(f"{'measure':<34} {'now':>14} {'limit':>14}  result")
    for name, base in baseline.items():
        if name not in current:
            print(f"{name:<34} {'':>14} {limit(base):>14.10g}  not measured yet")
            continue
        now = current[name]
        worse = now > limit(base) if base["better"] == "lower" else now < limit(base)
        failed |= worse
        print(f"{name:<34} {now:>14.10g} {limit(base):>14.10g}  {'FAIL' if worse else 'ok'}")
    for name in current.keys() - baseline.keys():
        failed = True
        print(f"{name:<34} {current[name]:>14.10g} {'':>14}  FAIL: not in the baseline")
    if failed:
        print(
            "\nA performance budget failed. Fast is a feature, so this change cannot merge as it is."
            "\nMeasure where the time or the bytes went, fix the cause, and run scripts/perf.sh again."
        )
        return 1
    print("\nEvery measured budget holds.")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: perf-gate.py BASELINE CURRENT", file=sys.stderr)
        sys.exit(2)
    sys.exit(main(sys.argv[1], sys.argv[2]))
