#!/bin/bash
# Run each both-site mutant of the base readout against the workspace suite and the board, then revert.
set -u
W=/Users/oxide/src/rumors-slot-07
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/reviewer-read-high-part
OI=/Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh
for kind in drop repeat; do
  python3 -I $S/mutate.py $kind apply || exit 2
  git -C $W diff --stat
  $OI $W 'unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; nice -n 10 cargo nextest run --workspace --all-features --locked --no-fail-fast; echo "TESTALL_EXIT=$?"; nice -n 10 just amp-board-acceptance; echo "BOARD_EXIT=$?"' > $S/mutant-$kind.log 2>&1
  echo "remote exit $?" >> $S/mutant-$kind.log
  python3 -I $S/mutate.py $kind revert || exit 3
  if [ -n "$(git -C $W diff)" ]; then echo "RESTORE FAILED"; exit 4; fi
  echo "$kind done, tree restored"
done
echo ALL-DONE
