#!/usr/bin/env bash
# Prove that every entry in the lint table matters (phase 0, task 0.3): remove
# each entry in turn from lints/lints.toml and from Cargo.toml, run
# scripts/check-lints.sh, and require it to fail. The two files are restored
# after every run, and on exit. Takes a few minutes; CI does not run it.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TABLE="$ROOT/lints/lints.toml"
WORKSPACE="$ROOT/Cargo.toml"
SAVED="$(mktemp -d "${TMPDIR:-/tmp}/prove-lint-table.XXXXXX")"
cp "$TABLE" "$SAVED/lints.toml"
cp "$WORKSPACE" "$SAVED/Cargo.toml"
restore() { cp "$SAVED/lints.toml" "$TABLE"; cp "$SAVED/Cargo.toml" "$WORKSPACE"; }
trap 'restore; rm -rf "$SAVED"' EXIT

# Entries are the "name = ..." lines inside the [lints.*] sections.
names=()
while IFS= read -r name; do names+=("$name"); done \
  < <(awk '/^\[lints\./ { on = 1; next } on && /^[a-z0-9_]+ *=/ { print $1 }' "$TABLE")

if ! "$ROOT/scripts/check-lints.sh" >/dev/null 2>&1; then
  echo "scripts/check-lints.sh fails on the full table. Fix that first."
  exit 1
fi

survivors=()
for name in "${names[@]}"; do
  # Remove the entry from the table and its mirror, which share the same lines.
  sed -i.bak "/^${name} *=/d" "$TABLE" "$WORKSPACE" && rm -f "$TABLE.bak" "$WORKSPACE.bak"
  if "$ROOT/scripts/check-lints.sh" >/dev/null 2>&1; then
    survivors+=("$name")
    echo "still passes without: $name"
  else
    echo "fails without: $name"
  fi
  restore
done

if [ "${#survivors[@]}" -gt 0 ]; then
  echo "These entries can be removed without the check noticing: ${survivors[*]}"
  echo "Add a marked violation for each one in lints/check/src/violations.rs."
  exit 1
fi
echo "Removing any one of the ${#names[@]} entries makes scripts/check-lints.sh fail."
