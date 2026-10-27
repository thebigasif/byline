#!/bin/bash
# Usage: run-track.sh <name> <worktree> <brief>. Runs one headless Claude Code builder in tmux.
# The builder gets builder-brief.md plus the task brief, an allowlist of commands, and no way to
# merge, force-push or use sudo. KHALID_OPS holds the prompts and logs (default ~/khalid-ops).
set -euo pipefail
OPS=${KHALID_OPS:-$HOME/khalid-ops}; HERE=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$OPS/briefs" "$OPS/logs"
name=$1; dir=$2; brief=$3
cd "$dir" && git fetch -q origin && git checkout -q -B "track/$name" origin/main 2>/dev/null || true
cat "$HERE/builder-brief.md" > "$OPS/briefs/$name.prompt"; echo >> "$OPS/briefs/$name.prompt"; cat "$brief" >> "$OPS/briefs/$name.prompt"
cat > "$OPS/briefs/$name.sh" <<INNER
#!/bin/bash
export CARGO_BUILD_JOBS=4
command -v sccache >/dev/null && export RUSTC_WRAPPER=sccache SCCACHE_DIR=$OPS/sccache SCCACHE_CACHE_SIZE=40G
cd "$dir"
claude -p "\$(cat "$OPS/briefs/$name.prompt")" --model claude-opus-5-5 --permission-mode acceptEdits \
  --allowedTools 'Bash(cargo *),Bash(just *),Bash(git *),Bash(gh pr create*),Bash(gh pr view*),Bash(gh pr checks*),Bash(gh run *),Bash(gh api repos/${KHALID_REPO:-thebigasif/khalid}/*),Bash(rustup *),Bash(ls *),Bash(cat *),Bash(grep *),Bash(rg *),Bash(find *),Bash(sed *),Bash(awk *),Bash(wc *),Bash(head *),Bash(tail *),Bash(diff *),Bash(mkdir *),Bash(cp *),Bash(mv *),Bash(typos*),Bash(taplo *),Bash(sqlite3 *),Bash(./scripts/*),Bash(scripts/*),Read,Edit,Write,Glob,Grep,WebFetch,WebSearch,Skill' \
  --disallowedTools 'Bash(gh pr merge*),Bash(git push --force*),Bash(git push -f*),Bash(sudo *),Bash(rm -rf /*)' \
  > "$OPS/logs/$name.log" 2>&1
echo "exit \$?" >> "$OPS/logs/$name.log"
# Build output grows by tens of GB per run: clear it when it passes 15 GB.
[ "\$(du -s target 2>/dev/null | cut -f1)" -gt 15000000 ] 2>/dev/null && rm -rf target
INNER
chmod +x "$OPS/briefs/$name.sh"
tmux kill-session -t "khalid-$name" 2>/dev/null || true
tmux new-session -d -s "khalid-$name" "$OPS/briefs/$name.sh"
echo "started khalid-$name in $dir"
