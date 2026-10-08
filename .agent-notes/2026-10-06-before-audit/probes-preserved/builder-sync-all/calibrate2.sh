#!/bin/bash
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/builder-sync-all
W=/Users/oxide/src/rumors-slot-34
R=/Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh
F='test(clock_and_list_laws) | test(laws_hold_on_organic_populations) | test(sync_all) | test(join_all) | test(conservation)'
run() {
  "$R" "$W" "unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; nice -n 10 cargo nextest run --locked -p before --all-features --lib --no-fail-fast -E \"$F\"" > "$S/$1.log" 2>&1
  echo "exit $?" >> "$S/$1.log"
}
board() {
  "$R" "$W" "unset CARGO_TARGET_DIR; export AMP_BOARD_SHARDS=24; nice -n 10 cargo run --locked --release -p before --example amp_board --features touch-meter,scan-meter,serde,borsh -- acceptance" > "$S/$1.log" 2>&1
  echo "exit $?" >> "$S/$1.log"
}
git -C $W diff | shasum > $S/diff-before-cal2.sha
run split-focused
python3 $S/mutant2.py apply M1 && run split-M1; python3 $S/mutant2.py revert M1
python3 $S/mutant2.py apply M2 && run split-M2; python3 $S/mutant2.py revert M2
python3 $S/mutant2.py apply M3 && board split-M3-board; python3 $S/mutant2.py revert M3
git -C $W diff | shasum > $S/diff-after-cal2.sha
echo done > $S/calibrate2.done
