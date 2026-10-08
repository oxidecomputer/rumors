#!/bin/bash
# usage: exp.sh <commit> <op_count> <op_pos>  -- base-scale column with the allocation log.
set -euo pipefail
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/reviewer-ceiling-rules
WT=$S/rumors-review-ceil
git -C $WT checkout -q --detach "$1"
python3 -I $S/patch_alloc.py $WT/crates/before/examples/amp_board.rs
{ echo "HEAD $(git -C $WT rev-parse HEAD) (amp_board.rs diagnostic patch applied)"; /Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh $WT "unset CARGO_TARGET_DIR; nice -n 10 cargo build --release --locked -p before --example amp_board --features touch-meter,scan-meter,serde,borsh; nice -n 10 target/release/examples/amp_board --shard $3/$2 --scale-bits 3ff0000000000000 >/dev/null"; echo "exit $?"; } > $S/x-$1.log 2>&1 || true
# Restore the scratch worktree's tracked file and confirm a clean tree.
git -C $WT checkout -- crates/before/examples/amp_board.rs
git -C $WT status --short
tail -1 $S/x-$1.log
grep -c '^DIAG' $S/x-$1.log || true
