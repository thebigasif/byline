#!/bin/bash
# Stack train. Lands every ready stack of pull requests with one squash commit and one CI run:
# retarget the top of each stack to main, resolve conflicts on the top only, wait for required checks
# (rerunning any check stuck over 15 minutes), print a review line per pull request, squash the top,
# close the lower pull requests as landed, retarget anything left on their branches, delete branches.
# Needs gh (signed in with merge rights), git, jq and python3. See docs/process/build-pipeline.md.
# KHALID_REPO: owner/name. KHALID_CLONE: a clone used to compare commits. KHALID_SCRATCH: a clone
# the stuck-check rerun may check out. APPROVED="n": stack tops whose lint suppressions a maintainer
# reviewed. ONLY="n": land only the stacks holding these pull requests.
HERE=$(cd "$(dirname "$0")" && pwd)
R=${KHALID_REPO:-thebigasif/khalid}
CLONE=${KHALID_CLONE:-$HOME/khalid}
SCRATCH=${KHALID_SCRATCH:-$HOME/khalid-scratch}
APPROVED=${APPROVED:-}; ONLY=${ONLY:-}
export KHALID_CLONE=$CLONE
gh pr list -R $R --state open --limit 300 --json number,baseRefName,headRefName,title,isDraft,author \
  --jq '[.[]|select(.isDraft==false and (.author.login|test("dependabot")|not))]' > /tmp/train-prs.json
num_of_head() { jq -r --arg h "$1" '.[]|select(.headRefName==$h)|.number' /tmp/train-prs.json; }
children() { jq -r --arg h "$1" '.[]|select(.baseRefName==$h)|.number' /tmp/train-prs.json; }
head_of() { jq -r --argjson n "$1" '.[]|select(.number==$n)|.headRefName' /tmp/train-prs.json; }
title_of() { jq -r --argjson n "$1" '.[]|select(.number==$n)|.title' /tmp/train-prs.json; }

# 1. Chains: from each pull request on main, follow single children upwards.
: > /tmp/train-chains
for b in $(jq -r '.[]|select(.baseRefName=="main")|.number' /tmp/train-prs.json | sort -n); do
  chain=$b; cur=$b
  while :; do kids=$(children "$(head_of $cur)"); [ $(echo "$kids" | grep -c .) = 1 ] || break; cur=$kids; chain="$chain $cur"; done
  echo "$chain" >> /tmp/train-chains
done
# A top retargeted to main by an earlier run still carries its lower stack: fold that stack in.
python3 "$HERE/train-fold.py"
# ONLY="n m": land just the chains holding these pull requests (an integration lands alone, first).
if [ -n "$ONLY" ]; then
  while read -r chain; do for n in $ONLY; do [[ " $chain " == *" $n "* ]] && { echo "$chain"; break; }; done; done < /tmp/train-chains > /tmp/train-only
  mv /tmp/train-only /tmp/train-chains
fi
echo "chains: $(wc -l < /tmp/train-chains)"; cat /tmp/train-chains | sed 's/^/  /'

# 2. Retarget tops to main; resolve conflicts on tops only.
while read -r chain; do
  top=${chain##* }; [ "$top" = "${chain%% *}" ] || gh api -X PATCH repos/$R/pulls/$top -f base=main >/dev/null 2>&1
done < /tmp/train-chains
sleep 15
while read -r chain; do
  top=${chain##* }
  for i in $(seq 1 12); do m=$(gh pr view $top -R $R --json mergeable --jq .mergeable); [ "$m" = UNKNOWN ] && sleep 10 || break; done
  [ "$m" = CONFLICTING ] && echo "  #$top resolve: $("$HERE/resolve.sh" "$(head_of $top)" 2>&1 | tail -1)"
done < /tmp/train-chains

# 3. Wait for every top's required checks, with the stuck-check watchdog.
declare -A state reruns
deadline=$(( $(date +%s) + 3600 ))
while [ $(date +%s) -lt $deadline ]; do
  pending=0
  while read -r chain; do
    top=${chain##* }; [ -n "${state[$top]}" ] && continue
    j=$(gh pr checks $top -R $R --required --json bucket,startedAt 2>/dev/null)
    [ -z "$j" ] || [ "$j" = "[]" ] && { pending=1; continue; }
    if echo "$j" | jq -e 'any(.[]; .bucket=="fail" or .bucket=="cancel")' >/dev/null; then state[$top]=red
    elif echo "$j" | jq -e 'all(.[]; .bucket=="pass")' >/dev/null; then state[$top]=green
    else
      pending=1
      if echo "$j" | jq -e --argjson now $(date +%s) 'any(.[]; .bucket=="pending" and .startedAt!=null and (.startedAt|fromdate) < ($now-900))' >/dev/null && [ "${reruns[$top]:-0}" -lt 2 ]; then
        reruns[$top]=$(( ${reruns[$top]:-0} + 1 ))
        (h=$(head_of $top); cd "$SCRATCH" && git fetch -q origin && git checkout -q -f -B "$h" "origin/$h" \
          && git commit -q --allow-empty -m "ci: rerun checks" && git push -q origin "$h") && echo "  #$top stuck check, rerun pushed"
      fi
    fi
  done < /tmp/train-chains
  [ $pending = 0 ] && break; sleep 20
done

# 4. Review and land.
while read -r chain; do
  top=${chain##* }; set -- $chain; n=$#
  echo "=== chain $chain: ${state[$top]:-timeout}"
  [ "${state[$top]}" = green ] || continue
  for i in $(seq 1 18); do m=$(gh pr view $top -R $R --json mergeable --jq .mergeable); [ "$m" = UNKNOWN ] && sleep 10 || break; done
  [ "$m" = MERGEABLE ] || { echo "    mergeable: $m, skipped"; continue; }
  bad=""; body=""
  for p in $chain; do
    gh pr diff $p -R $R > /tmp/pr$p.diff 2>/dev/null
    sup=$(awk '/^diff --git/{f=$3} /^\+.*#\[(allow|expect)/{print f}' /tmp/pr$p.diff | grep -v '/tests/' | grep -v lints/check | sort -u | tr '\n' ' ')
    echo "    #$p $(title_of $p) | tests $(grep -cE '^\+\s*#\[(tokio::|sqlx::)?test\]|^\+\s*proptest!' /tmp/pr$p.diff); crates $(grep -oE '^\+\+\+ b/crates/[a-z-]+' /tmp/pr$p.diff | sort -u | sed 's#+++ b/crates/##' | tr '\n' ' '); suppressions: ${sup:-none}"
    [ -n "$sup" ] && bad=1
    body="$body- $(title_of $p) (#$p)"$'\n'
  done
  [ -n "$bad" ] && [[ " $APPROVED " != *" $top "* ]] && { echo "    suppressions outside tests, chain left for review"; continue; }
  if [ $n = 1 ]; then gh pr merge $top -R $R --squash >/dev/null 2>&1 || { echo "    merge failed"; continue; }
  else
    gh pr merge $top -R $R --squash --subject "$(title_of $1) and $((n-1)) more (#$top)" \
      --body "$body"$'\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>' >/dev/null 2>&1 || { echo "    merge failed"; continue; }
  fi
  echo "    MERGED as one commit"
  for p in $chain; do
    h=$(head_of $p)
    if [ $p != $top ]; then
      git -C "$CLONE" fetch -q origin +refs/pull/$p/head:refs/train/$p +refs/pull/$top/head:refs/train/$top
      git -C "$CLONE" fetch -q origin main; if [ -z "$(git -C "$CLONE" rev-list --no-merges refs/train/$p ^refs/train/$top ^origin/main)" ]; then
        gh pr close $p -R $R --comment "Landed on main as part of #$top." >/dev/null 2>&1
      else
        gh api -X PATCH repos/$R/pulls/$p -f base=main >/dev/null 2>&1; echo "    #$p not inside #$top, left open on main"; continue
      fi
    fi
    for c in $(gh pr list -R $R --state open --limit 300 --json number,baseRefName --jq ".[]|select(.baseRefName==\"$h\")|.number"); do
      gh api -X PATCH repos/$R/pulls/$c -f base=main >/dev/null 2>&1; echo "    #$c retargeted to main"; done
    gh api -X DELETE repos/$R/git/refs/heads/$h >/dev/null 2>&1
  done
done < /tmp/train-chains
echo "open total: $(gh pr list -R $R --state open --limit 300 --json number --jq length)"
