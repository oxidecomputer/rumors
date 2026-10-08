#!/usr/bin/env python3
"""Apply each mutant as a unique string swap, run a command on the box, revert.

Usage: mutate.py <mutants.json> <remote command> <outdir>
"""
import json, subprocess, sys, os
WT = "/Users/oxide/src/rumors-audit-l2-algebra"
WRAP = "/Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh"
mutants = json.load(open(sys.argv[1]))
cmd = sys.argv[2]
outdir = sys.argv[3]
os.makedirs(outdir, exist_ok=True)
summary = open(os.path.join(outdir, "summary.txt"), "a")
for m in mutants:
    path = os.path.join(WT, m["file"])
    src = open(path).read()
    n = src.count(m["old"])
    if n != 1:
        summary.write(f"{m['name']}: SKIP (old occurs {n} times)\n"); summary.flush(); continue
    open(path, "w").write(src.replace(m["old"], m["new"]))
    try:
        log = os.path.join(outdir, m["name"] + ".log")
        with open(log, "w") as f:
            rc = subprocess.call([WRAP, WT, cmd], stdout=f, stderr=subprocess.STDOUT)
    finally:
        open(path, "w").write(src)
    clean = subprocess.call(["git", "-C", WT, "diff", "--quiet"]) == 0
    verdict = "KILLED" if rc != 0 else "SURVIVED"
    summary.write(f"{m['name']}: {verdict} rc={rc} restored={clean}\n"); summary.flush()
    if not clean:
        summary.write("ABORT: tree not restored\n"); break
