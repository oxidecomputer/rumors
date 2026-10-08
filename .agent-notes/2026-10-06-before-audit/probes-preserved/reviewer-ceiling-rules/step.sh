#!/bin/bash
# usage: step.sh <commit>  -- check out <commit> in the scratch worktree, derive the
# board's op table positions and features there, measure, and print comb readings.
set -euo pipefail
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/reviewer-ceiling-rules
WT=$S/rumors-review-ceil
c=$1
git -C $WT checkout -q --detach "$c"
ops=$(git -C $WT ls-files crates/before/src | grep 'board/ops.rs$')
read n p1 p2 < <(awk '/fn ops\(\) -> Vec<Op>/{on=1} on && /name: "/{match($0,/"[^"]*"/); nm=substr($0,RSTART+1,RLENGTH-2); if (nm=="own_version_to_version") a=n; if (nm=="clock_own_version_to_version") b=n; n++} END{print n, a, b}' $WT/$ops)
feats=$(awk '/name = "amp_board"/{getline; print}' $WT/crates/before/Cargo.toml | sed 's/required-features = \[//; s/\]//; s/[" ]//g')
echo "commit $c ops=$ops n=$n p1=$p1 p2=$p2 feats=$feats"
$S/measure.sh "$c" "$n" "$p1" "$p2" "$feats"
log=$S/m-$c.log
tail -1 $log
others=$(grep '^cell' $log | cut -f2 | sort -u | grep -v -x -e own_version_to_version -e clock_own_version_to_version || true)
[ -z "$others" ] || echo "WARNING: shard emitted other ops: $others"
python3 -I $S/comb.py $log
