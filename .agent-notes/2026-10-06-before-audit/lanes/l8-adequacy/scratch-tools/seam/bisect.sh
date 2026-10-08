#!/usr/bin/env bash
# Bisect the seam_stop diff_large rise over seam/commits.txt (old = 7867 at index -1, new = 8892 at the last).
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l8
W=/Users/oxide/src/rumors-audit-l8-adequacy
mapfile -t C < $S/seam/commits.txt
lo=-1; hi=$((${#C[@]} - 1))
while [ $((hi - lo)) -gt 1 ]; do
  mid=$(((lo + hi) / 2)); sha=${C[$mid]}
  "$W/l8/mkprobe-at.sh" "$sha" >/dev/null
  /Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh "$W" "unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=2; bash l8/seam.sh $sha" > "$S/seam/$sha.log" 2>&1
  v=$(grep -o 'diff_large=[0-9-]*' "$S/seam/$sha.log" | head -1 | cut -d= -f2)
  echo "$sha index $mid diff_large=$v" >> "$S/seam/bisect.txt"
  case "$v" in
    7867) lo=$mid ;;
    8892) hi=$mid ;;
    *) echo "unexpected reading at $sha: '$v'" >> "$S/seam/bisect.txt"; break ;;
  esac
done
echo "first new: ${C[$hi]} (index $hi); last old index $lo" >> "$S/seam/bisect.txt"
"$W/l8/mkprobe.sh" >/dev/null
