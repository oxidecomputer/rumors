#!/bin/bash
# usage: measure.sh <label> <op_count> <pos1> [<pos2>] [features]
# Builds the release board at the scratch worktree's HEAD on ox-east-1 and emits
# the shard-child rows for the given op columns at base and top scale.
set -u
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/reviewer-ceiling-rules
WT=$S/rumors-review-ceil
label=$1; n=$2; p1=$3; p2=${4:-}; feats=${5:-touch-meter,scan-meter}
runs=""
for s in 3ff0000000000000 4010000000000000; do for p in $p1 $p2; do runs="$runs nice -n 10 target/release/examples/amp_board --shard $p/$n --scale-bits $s;"; done; done
{ echo "HEAD $(git -C $WT rev-parse HEAD)"; /Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh $WT "unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; nice -n 10 cargo build --release --locked -p before --example amp_board --features $feats; $runs"; echo "exit $?"; } > $S/m-$label.log 2>&1
