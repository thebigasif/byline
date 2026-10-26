#!/usr/bin/env bash
# Prove the lint table fires (RULES.md BL-LNT-3).
#
# lints/lints.toml is the table. The root Cargo.toml mirrors it under
# [workspace.lints.*]. lints/check/src/violations.rs holds one deliberate
# violation per entry, marked in the source:
#   // expect: <lint>   the lint must fire on the marked line
#   // quiet: <lint>    the table allows the lint, so the marked line stays silent
# A marker on a line of its own applies to the next code line.
#
# The check fails when the two tables drift, when an entry has no marker or a
# marker names no entry, when a marked lint stays silent, or when anything
# unmarked fires. Clippy proves the rustc and clippy lints, rustdoc proves the
# rustdoc lints. Both read the root clippy.toml, like every crate here.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TABLE="$ROOT/lints/lints.toml"
WORKSPACE="$ROOT/Cargo.toml"
RULES="$ROOT/docs/standards/RULES.md"
SRC="$ROOT/lints/check/src/violations.rs"

for tool in cargo python3; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "check-lints: $tool is not on PATH. Install it, then run this script again." >&2
    exit 1
  fi
done

# A target directory of its own, so the cfg below never invalidates the normal
# build cache. It is emptied each run so cached diagnostics cannot hide a change.
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}/lint-check"
rm -rf "$TARGET"
mkdir -p "$TARGET"

export CARGO_TERM_COLOR=never
export CLIPPY_CONF_DIR="$ROOT"
# Replaces any -D warnings from the caller: this script judges every diagnostic itself.
export RUSTFLAGS="--cfg byline_lint_check"
export RUSTDOCFLAGS="--cfg byline_lint_check"

set +e
cargo clippy --manifest-path "$WORKSPACE" -p lint-check --target-dir "$TARGET" \
  --message-format=json >"$TARGET/clippy.json" 2>"$TARGET/clippy.err"
CLIPPY_STATUS=$?
cargo doc --manifest-path "$WORKSPACE" -p lint-check --no-deps --target-dir "$TARGET" \
  --message-format=json >"$TARGET/doc.json" 2>"$TARGET/doc.err"
DOC_STATUS=$?
set -e

python3 - "$TABLE" "$WORKSPACE" "$RULES" "$SRC" "$TARGET" "$CLIPPY_STATUS" "$DOC_STATUS" <<'PY'
import json
import re
import sys
import tomllib
from pathlib import Path

table_path, workspace_path, rules_path, src_path, target, clippy_s, doc_s = sys.argv[1:]
target = Path(target)
failed = False


def fail(text):
    global failed
    failed = True
    print(text)


# ------------------------------------------------------------- the two tables

PREFIX = {"rust": "", "rustdoc": "rustdoc::", "clippy": "clippy::"}


def entries(lints):
    """The table as an ordered list of (full lint name, level)."""
    out = []
    for tool, table in lints.items():
        for name, value in table.items():
            level = value["level"] if isinstance(value, dict) else value
            out.append((PREFIX.get(tool, f"{tool}::") + name, level, json.dumps(value, sort_keys=True)))
    return out


table = entries(tomllib.loads(Path(table_path).read_text())["lints"])
mirror = entries(tomllib.loads(Path(workspace_path).read_text())["workspace"]["lints"])
if table != mirror:
    fail("The lint table drifted: [workspace.lints] in Cargo.toml must list the same lints, levels and order as lints/lints.toml.")
    for row in sorted(set(table) ^ set(mirror)):
        side = "lints/lints.toml" if row in table else "Cargo.toml"
        print(f"  only in {side}: {row[0]} = {row[1]}")
level_of = {name: level for name, level, _ in table}

# Rule IDs from the Enforced column of RULES.md, for the report.
rule_of = {}
for line in Path(rules_path).read_text().splitlines():
    match = re.match(r"^\| (BL-[A-Z]+-\d+) \|", line)
    if match:
        for lint in re.findall(r"`([a-z0-9_:]+)`", line.rsplit("|", 2)[-2]):
            rule_of.setdefault(lint.split("::")[-1], match.group(1))

# ----------------------------------------------------------------- the markers

MARKER = re.compile(r"(?:^|\s)// (expect|quiet): ([a-z0-9_:]+)\s*$")
src_lines = Path(src_path).read_text().splitlines()


def marked_line(line_no):
    """A marker after code marks its own line; a marker alone marks the next code line."""
    if MARKER.sub("", src_lines[line_no - 1]).strip():
        return line_no
    for j in range(line_no, len(src_lines)):
        stripped = src_lines[j].strip()
        if stripped and not stripped.startswith("//"):
            return j + 1
    return line_no


markers = []
for i, line in enumerate(src_lines, start=1):
    match = MARKER.search(line)
    if match:
        markers.append((marked_line(i), match.group(1), match.group(2)))

for _, kind, lint in markers:
    level = level_of.get(lint)
    if level is None:
        fail(f"The marker `{kind}: {lint}` names a lint that is not in lints/lints.toml.")
    elif (kind == "quiet") != (level == "allow"):
        fail(f"The marker `{kind}: {lint}` does not match the table, which sets it to {level}.")
for name, level, _ in table:
    want = "quiet" if level == "allow" else "expect"
    if not any(kind == want and lint == name for _, kind, lint in markers):
        fail(f"The table entry {name} = {level} has no `// {want}: {name}` marker in lints/check.")

# ----------------------------------------------------------- the diagnostics

W_NOTE = re.compile(r"-[WDF] ([A-Za-z0-9_:-]+)")
ATTR_NOTE = re.compile(r"#\[(?:warn|deny|forbid)\(([A-Za-z0-9_:]+)\)\]")
SUMMARY = re.compile(r"^(\d+ warnings? emitted|aborting due to|could not (compile|document)|warning: build failed)")


def names_of(message):
    """Every lint name a diagnostic answers to, including the group that enabled it."""
    raw = [(message.get("code") or {}).get("code") or ""]
    for child in message.get("children") or []:
        note = child.get("message") or ""
        raw += W_NOTE.findall(note) + ATTR_NOTE.findall(note)
    return {name.replace("-", "_") for name in raw if name and not re.fullmatch(r"E\d+", name)}


def diagnostics(path):
    out = []
    for line in path.read_text().splitlines():
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        if obj.get("reason") != "compiler-message":
            continue
        message = obj.get("message") or {}
        if message.get("level") not in ("warning", "error"):
            continue
        text = (message.get("message") or "").split("\n")[0]
        spans = message.get("spans") or []
        span = next((s for s in spans if s.get("is_primary")), spans[0] if spans else {})
        if not span and SUMMARY.match(text):
            continue
        out.append((Path(span.get("file_name") or "?").name, span.get("line_start"), names_of(message), text))
    return out


clippy = [d for d in diagnostics(target / "clippy.json") if not any(n.startswith("rustdoc::") for n in d[2])]
rustdoc = [d for d in diagnostics(target / "doc.json") if any(n.startswith("rustdoc::") for n in d[2])]

for status, name, found in ((int(clippy_s), "clippy", clippy), (int(doc_s), "doc", rustdoc)):
    if status != 0 and not any(text for *_, text in found):
        fail(f"cargo {name} failed without a diagnostic:")
        print((target / f"{name}.err").read_text().strip())

fired = clippy + rustdoc
used = set()
rows = []
for line_no, kind, lint in markers:
    if kind != "expect":
        continue
    hit = next(
        (i for i, (file, at, names, _) in enumerate(fired)
         if i not in used and file == "violations.rs" and at == line_no and lint in names),
        None,
    )
    if hit is None:
        rows.append((lint, f"MISSING on violations.rs:{line_no}"))
        failed = True
    else:
        used.add(hit)
        rows.append((lint, "fired"))

width = max(len(lint) for lint, _ in rows) if rows else 4
print(f"{'rule':<10}  {'lint':<{width}}  status")
print(f"{'-' * 10}  {'-' * width}  ------")
for lint, status in rows:
    print(f"{rule_of.get(lint.split('::')[-1], ''):<10}  {lint:<{width}}  {status}")

unexpected = [d for i, d in enumerate(fired) if i not in used]
if unexpected:
    fail("\nDiagnostics with no marker:")
    for file, at, names, text in unexpected:
        print(f"  {file}:{at}: {', '.join(sorted(names)) or 'no lint name'}: {text}")

if failed:
    print("\nThe lint table is not proven. Fix the table or the markers in lints/check, then run this script again.")
    sys.exit(1)
quiet = sum(1 for _, kind, _ in markers if kind == "quiet")
print(f"\n{len(rows)} expected lints fired, {quiet} allowed lints stayed quiet, and the table matches Cargo.toml.")
PY
