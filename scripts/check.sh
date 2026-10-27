#!/usr/bin/env bash
# Everything CI checks, in the same order, so a green local run means a green CI.
# Each step prints its name. The first failure stops the run.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

step() { printf '\n==> %s\n' "$1"; }

step "No em dashes in any tracked text (RULES.md BL-TXT-1)"
if git grep -nI -- $'\xe2\x80\x94' -- ':!*.lock' ':!.claude/skills/rust-best-practices/references/*' >/dev/null; then
  git grep -nI -- $'\xe2\x80\x94' -- ':!*.lock' ':!.claude/skills/rust-best-practices/references/*'
  echo "Replace each em dash with a full stop, a comma or a colon."
  exit 1
fi

step "Spelling (typos)"
typos

if ls crates/*/Cargo.toml >/dev/null 2>&1; then
  step "Format (cargo fmt --check)"
  cargo fmt --all -- --check

  step "Clippy, warnings as errors"
  cargo clippy --workspace --all-targets --all-features -- -D warnings

  step "Tests (nextest + doc tests)"
  cargo nextest run --workspace --all-features
  cargo test --workspace --doc

  step "Docs, warnings as errors"
  RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features

  step "Dependencies (cargo deny)"
  cargo deny check

  step "Unused dependencies (cargo machete)"
  cargo machete

  step "Lint table proven to fire"
  ./scripts/check-lints.sh

  step "anyhow stays out of library crates (BL-ERR-1)"
  if git grep -nE 'anyhow' -- 'crates/*/src/**' ':!crates/khalid/**' >/dev/null; then
    git grep -nE 'anyhow' -- 'crates/*/src/**' ':!crates/khalid/**'
    echo "Libraries use their own thiserror enums. Only crates/khalid may wrap errors."
    exit 1
  fi

  step "Disallowed methods and types are suppressed only where allowed (BL-RUN-1, BL-RUN-2, BL-RUN-8, BL-SEC-4)"
  # The clock, config, outbound and path modules may each suppress the rule
  # once, on the one line that has to break it. Nothing else may.
  allowed='crates/khalid-core/src/clock.rs crates/khalid-core/src/config.rs crates/khalid-core/src/site_path.rs crates/khalid-outbound/src/outbound.rs'
  bad=$(git grep -nE 'clippy::disallowed_(methods|types)' -- 'crates/**/*.rs' | grep -vE "^($(echo "$allowed" | tr ' ' '|')):" || true)
  for file in $allowed; do
    if [ -f "$file" ] && [ "$(grep -cE 'clippy::disallowed_(methods|types)' "$file")" -gt 1 ]; then
      bad="$bad"$'\n'"$file: more than one suppression"
    fi
  done
  if [ -n "${bad//[$'\n']/}" ]; then
    echo "$bad"
    echo "Read time through Clock, settings through Config, the network through Outbound and paths through SitePath."
    exit 1
  fi

  step "File size (BL-SIZE-1): hand-written Rust files under 600 lines"
  over=$(git ls-files 'crates/*/src/*.rs' 'crates/*/src/**/*.rs' | grep -v '/generated/' | xargs -I{} awk 'END{ if (NR>600) print FILENAME": "NR" lines" }' {} || true)
  if [ -n "$over" ]; then echo "$over"; echo "Split the file (BL-SIZE-1)."; exit 1; fi
else
  echo "No crates yet. Rust checks start with the first crate (Phase 0)."
fi

if [ -f admin/package.json ]; then
  step "Admin: Biome, TypeScript, Vitest"
  # Run only the tools pinned in admin/node_modules. A bare npx would download and run
  # whatever unrelated package carries the same name when node_modules is missing.
  if [ -d admin/node_modules ]; then
    (cd admin && npm exec --no -- biome ci . && npm exec --no -- tsc --noEmit && npm exec --no -- vitest run)
  else
    echo "admin/node_modules is missing, so the admin checks are left to CI. Run npm ci in admin/ to run them here."
  fi
fi

printf '\nAll checks passed.\n'
