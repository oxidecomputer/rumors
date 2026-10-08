#!/usr/bin/env bash
# Apply each wasm32 pin injection to a fresh probe tree, run its pin, record.
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l8
W=/Users/oxide/src/rumors-audit-l8-adequacy
ids=$(python3 -c "import sys; sys.path.insert(0,'$S/w32'); from injections import INJ; print(' '.join(i[0] for i in INJ))")
for id in $ids; do
  "$W/l8/mkprobe.sh" > /dev/null
  filter=$(python3 - "$id" <<'PY'
import sys
sys.path.insert(0, "/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l8/w32")
from injections import INJ
want = sys.argv[1]
for id_, f, swaps, flt in INJ:
    if id_ != want: continue
    p = "/Users/oxide/src/rumors-audit-l8-adequacy/l8/probe/crates/" + f
    s = open(p).read()
    for old, new, n in swaps:
        c = s.count(old)
        if c != n:
            print(f"COUNT-MISMATCH {old!r}: {c} != {n}", file=sys.stderr); sys.exit(3)
        s = s.replace(old, new)
    open(p, "w").write(s)
    print(flt)
PY
)
  if [ $? -ne 0 ]; then echo "$id: injection failed to apply" >> "$S/w32/verdicts.txt"; continue; fi
  ( cd "$W/l8/probe" && git -C "$W" diff --no-index --stat /dev/null /dev/null >/dev/null 2>&1 ); 
  /Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh "$W" "bash l8/w32.sh $id '$filter'" > "$S/w32/$id.log" 2>&1
  echo "$id: local exit=$? $(grep -E 'pins exit=|FAILED$' "$S/w32/$id.log" | tail -1)" >> "$S/w32/verdicts.txt"
done
"$W/l8/mkprobe.sh" > /dev/null
echo "all injections done; probe tree restored" >> "$S/w32/verdicts.txt"
