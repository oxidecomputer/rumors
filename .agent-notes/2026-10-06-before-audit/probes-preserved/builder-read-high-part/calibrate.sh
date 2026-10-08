#!/bin/bash
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/builder-read-high-part
WT=/Users/oxide/src/rumors-slot-07
for site in neg pos; do
  for kind in drop_push push_minus_one drop_touch; do
    log=$S/mutant-$site-$kind.log
    python3 -I $S/mutate.py $site $kind apply > $log 2>&1 || { echo "apply failed $site $kind"; exit 1; }
    git -C $WT diff >> $log
    /Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh $WT 'unset CARGO_TARGET_DIR; nice -n 10 cargo nextest run -p suanpan --all-features --locked --build-jobs 24 --test-threads 24 --no-fail-fast' >> $log 2>&1
    echo "exit=$?" >> $log
    python3 -I $S/mutate.py $site $kind revert >> $log 2>&1 || { echo "revert failed $site $kind"; exit 1; }
    if [ -n "$(git -C $WT diff)" ]; then echo "TREE NOT RESTORED after $site $kind"; exit 1; fi
    echo "$site $kind: $(grep -E '^\s+Summary' $log) $(tail -1 $log)"
  done
done
echo "final git diff:"; git -C $WT diff --stat; echo "(end)"
