#!/bin/bash
# Usage: calibrate_f.sh <nextest filterset> <mutant>...
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l7
WT=/Users/oxide/src/rumors-audit-l7-suanpan
FILTER="$1"; shift
TAG="$(echo "$FILTER" | tr -c 'a-zA-Z0-9' '_' | cut -c1-24)"
for m in "$@"; do
  python3 $S/mutate.py $m apply || { echo "$m APPLY-FAILED"; exit 1; }
  /Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh $WT "unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; L7_CASES=2000 nice -n 10 cargo nextest run --locked -j 24 -p suanpan --features touch-meter --no-fail-fast -E '$FILTER'" > $S/mut/$m.$TAG.log 2>&1
  status=$?
  python3 $S/mutate.py $m revert || { echo "$m REVERT-FAILED"; exit 1; }
  if ! git -C $WT diff --quiet; then echo "$m TREE-DIRTY-AFTER-REVERT"; exit 1; fi
  failed=$(grep -E '^\s+(FAIL|TIMEOUT|SIGABRT)' $S/mut/$m.$TAG.log | sed -E 's/.*suanpan //' | sort -u | tr '\n' ';')
  summary=$(grep -E 'Summary' $S/mut/$m.$TAG.log | sed -E 's/ +/ /g')
  echo "$m exit=$status [$summary] failed: $failed"
done
