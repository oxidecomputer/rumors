#!/usr/bin/env python3
"""Turn selected L8 survivor entries into git-applicable patches."""
import re, sys, os
D = "/Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/lanes/l8-adequacy/round-2/survivors/diffs"
OUT = sys.argv[1]
want = {
  # version group
  "version-1", "version-2", "version-3",
}
sel_names = [l.strip() for l in open(sys.argv[2]) if l.strip() and not l.startswith("#")]
entries = {}
for group in ["version", "span", "rest"]:
    text = open(f"{D}/{group}.md").read()
    for m in re.finditer(r"## (\S+): `([^`]+)`\n\n- name: `([^`]+)`.*?```diff\n(.*?)```", text, re.S):
        tag, loc, name, diff = m.groups()
        entries[name] = (tag, loc, diff)
for i, name in enumerate(sel_names):
    tag, loc, diff = entries[name]
    lines = diff.splitlines()
    path = lines[0][4:].strip()
    body = "\n".join(lines[2:]) + "\n"
    patch = f"--- a/{path}\n+++ b/{path}\n{body}"
    fname = f"{OUT}/{i:02d}-{tag}.patch"
    open(fname, "w").write(f"# {name}\n" if False else patch)
    print(f"{i:02d}-{tag}\t{name}")
