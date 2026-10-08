#!/usr/bin/env bash
# Fetch a campaign's missed and timeout lists from the box and write one
# survivors file per source module under the scratch survivors/ directory.
# Usage: split-survivors.sh <remote mutants.out dir> [<label>]
set -euo pipefail
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l8
remote=$1
label=${2:-campaign}
raw=$S/survivors/raw-$label
mkdir -p "$raw"
for f in missed timeout caught unviable; do
  ssh ox-east-1-agent "cat $remote/$f.txt 2>/dev/null || true" > "$raw/$f.txt"
done
# One file per module: MISSED and TIMEOUT lines, tagged.
python3 - "$S" "$raw" "$label" <<'PY'
import sys, os, collections
S, raw, label = sys.argv[1:4]
per = collections.defaultdict(list)
for tag in ("missed", "timeout"):
    for line in open(os.path.join(raw, f"{tag}.txt")):
        line = line.rstrip("\n")
        if not line:
            continue
        path = line.split(":", 1)[0]
        per[path].append(f"{tag.upper():8} {line}")
outdir = os.path.join(S, "survivors")
for path, lines in per.items():
    name = path.replace("crates/", "").replace("/src/", "/").replace("/", "__").replace(".rs", "") + ".txt"
    with open(os.path.join(outdir, name), "w") as f:
        f.write(f"# survivors of {path} ({label}); MISSED = no test failed, TIMEOUT = hung\n")
        f.write("\n".join(sorted(lines, key=lambda l: int(l.split(":")[1]) if l.split(":")[1].isdigit() else 0)) + "\n")
counts = {t: sum(1 for _ in open(os.path.join(raw, f"{t}.txt"))) for t in ("caught","missed","timeout","unviable")}
print(label, counts)
PY
