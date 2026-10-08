#!/bin/bash
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/builder-read-high-part
WT=/Users/oxide/src/rumors-slot-07
R=/Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh
$R $WT 'unset CARGO_TARGET_DIR; nice -n 10 cargo clippy -p suanpan --all-targets --all-features --locked -j 24 -- -D warnings && nice -n 10 cargo nextest run -p suanpan --all-features --locked --build-jobs 24 --test-threads 24 --no-fail-fast' > $S/base-unmutated.log 2>&1
echo "unmutated: exit=$? $(grep -E '^\s+Summary' $S/base-unmutated.log)"
for site in neg pos; do
  log=$S/base-mutant-$site-drop_touch.log
  python3 -I $S/mutate_base.py $site apply > $log 2>&1 || { echo "apply failed $site"; exit 1; }
  git -C $WT diff -- crates/suanpan/src/accumulator/digits/read.rs >> $log
  $R $WT 'unset CARGO_TARGET_DIR; nice -n 10 cargo nextest run -p suanpan --all-features --locked --build-jobs 24 --test-threads 24 --no-fail-fast' >> $log 2>&1
  echo "exit=$?" >> $log
  python3 -I $S/mutate_base.py $site revert >> $log 2>&1 || { echo "revert failed $site"; exit 1; }
  if [ -n "$(git -C $WT diff -- crates/suanpan/src/accumulator/digits/read.rs)" ]; then echo "READ.RS NOT RESTORED after $site"; exit 1; fi
  echo "base $site drop_touch: $(grep -E '^\s+Summary' $log)"
done
echo "(end)"
