#!/bin/bash
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l7
WT=/Users/oxide/src/rumors-audit-l7-suanpan
for m in "$@"; do
  python3 $S/mutate.py $m apply || { echo "$m APPLY-FAILED"; exit 1; }
  /Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh $WT 'unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; L7_CASES=2000 nice -n 10 cargo nextest run --locked -j 24 -p suanpan --features touch-meter --no-fail-fast -E "test(/l7_/)"' > $S/mut/$m.log 2>&1
  status=$?
  python3 $S/mutate.py $m revert || { echo "$m REVERT-FAILED"; exit 1; }
  if ! git -C $WT diff --quiet; then echo "$m TREE-DIRTY-AFTER-REVERT"; exit 1; fi
  echo "$m exit=$status $(grep -E '^\s+(PASS|FAIL|SIGABRT|SIGSEGV)' $S/mut/$m.log | sed -E 's/ +/ /g' | tr '\n' ';')"
done
