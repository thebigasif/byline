#!/bin/bash
# Usage: resolve.sh <branch>. Merges origin/main into a pull request branch and pushes normally.
# Only mechanical conflicts are resolved: both sides of a crate's Cargo.toml dependency list are
# kept. Any conflict in code, tests or fixtures is left for a builder, because taking one side
# silently drops the other side's work. Cargo.lock always starts from main's and cargo adds the
# branch's entries. Phase briefs, the ADR index and CHANGELOG.md take main's copy (the maintainer
# owns them); other docs are union-merged. Anything else conflicting is left for the author.
set -uo pipefail
branch=$1; W=${KHALID_SCRATCH:-$HOME/khalid-scratch}
[ -d "$W/.git" ] || git clone -q "https://github.com/${KHALID_REPO:-thebigasif/khalid}" "$W"
cd "$W"
clean() { rm -f .git/index.lock; git merge --abort 2>/dev/null || true; git reset -q --hard; git clean -fdq; }
union() { git show :2:"$1" > /tmp/o; git show :3:"$1" > /tmp/t; git show :1:"$1" > /tmp/b 2>/dev/null || : > /tmp/b; git merge-file -p --union /tmp/o /tmp/b /tmp/t > "$1" || true; git add "$1"; }
attempt() {
  clean; git fetch -q origin; git checkout -q -f -B "$branch" "origin/$branch" || return 9
  if git merge -q --no-edit origin/main >/dev/null 2>&1; then merged=clean; else
    merged=conflict
    for f in $(git diff --name-only --diff-filter=U); do
      case "$f" in
        Cargo.lock) git checkout --theirs -- Cargo.lock; git add Cargo.lock;;
        docs/phases/*.md|docs/adr/README.md|CHANGELOG.md) git checkout --theirs -- "$f"; git add "$f";;
        docs/*.md|changelog.d/*) union "$f";;
        crates/*/Cargo.toml|crates/khalid-lint/catalogue.toml) union "$f";;
        *) echo "UNRESOLVED: $f"; clean; return 2;;
      esac
    done
  fi
  git checkout -q origin/main -- Cargo.lock 2>/dev/null || true
  cargo check -q --workspace --all-targets >/dev/null 2>&1 || return 3
  git add -A Cargo.lock
  if [ "$merged" = conflict ]; then git commit -q -m "Merge main into $branch"
  elif ! git diff --cached --quiet; then git commit -q --amend --no-edit; fi
  return 0
}
attempt union; rc=$?
[ $rc = 0 ] && { git push -q origin "$branch" && echo "pushed $branch"; exit 0; }
[ $rc = 2 ] && exit 2
clean; echo "cargo check failed after the merge"; exit 3
