"""Fold a stack back together when an earlier run retargeted its top to main.

Chain A belongs under chain B when every real commit of A's top is in B's top or on main; A is prepended so
the train lands both with one squash. Anything else stays a separate chain.
"""
import os
import subprocess

REPO = os.environ.get("KHALID_CLONE", os.path.expanduser("~/khalid"))
chains = [line.split() for line in open("/tmp/train-chains") if line.strip()]


def git(*args):
    return subprocess.run(["git", "-C", REPO, *args], capture_output=True, text=True)


git("fetch", "-q", "origin", "main", *[f"+refs/pull/{c[-1]}/head:refs/train/{c[-1]}" for c in chains])
head = {c[-1]: git("rev-parse", f"refs/train/{c[-1]}").stdout.strip() for c in chains}


def under(a, b):
    # Every real commit of A is in B or already on main; merge commits from repairs do not count.
    missing = git("rev-list", "--no-merges", head[a[-1]], "^" + head[b[-1]], "^origin/main").stdout
    return head[a[-1]] != head[b[-1]] and not missing.strip()


changed = True
while changed:
    changed = False
    for a in chains:
        b = next((b for b in chains if b is not a and under(a, b)), None)
        if b is not None:
            b[:0] = a
            chains.remove(a)
            changed = True
            break
with open("/tmp/train-chains", "w") as f:
    f.writelines(" ".join(c) + "\n" for c in chains)
