#!/bin/bash
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/builder-events-cogen
W=/Users/oxide/src/rumors-slot-29
WRAP=/Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh
TICKSET='test(/version::tick::/) | test(/min_ticks/) | test(/ticks/)'
run() { # name filter
  m=$1; f=$2
  python3 $S/mutate.py $m apply || exit 1
  $WRAP $W "unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; nice -n 10 cargo nextest run --locked -p before --all-features --no-fail-fast -E '$f' || echo \"status \$?\"" > $S/calib-$m.log 2>&1
  echo "wrapper exit $?" >> $S/calib-$m.log
  mkdir -p $S/seeds-$m; rsync -a ox-east-1-agent:src/rumors-slot-29/crates/before/proptest-regressions/ $S/seeds-$m/
  python3 $S/mutate.py $m revert || exit 1
  if git -C $W diff --quiet && git -C $W diff --cached --quiet; then echo "$m restored: git diff empty" >> $S/calib-$m.log; else echo "$m NOT RESTORED" >> $S/calib-$m.log; exit 1; fi
}
for m in M6 M8 M15 M19; do run $m "$TICKSET"; done
run TAKE 'test(/tick::memo::/)'
run CENSUS 'test(/co_generated_regimes/)'
echo ALL_DONE > $S/calib-done
