#!/usr/bin/env bash
# Dump every amplification-board cell's raw samples (shard wire form) at the
# three scales the acceptance run measures, sorted, for a base/change diff.
# Runs on ox-east-1 inside ~/src/rumors-slot-03. Usage: dump-readings.sh <label>
set -euo pipefail
label=$1
cd "$HOME/src/rumors-slot-03"
unset CARGO_TARGET_DIR
nice -n 10 cargo build --locked -j 24 --release -p before --example amp_board \
    --features touch-meter,scan-meter,serde,borsh
exe=target/release/examples/amp_board
out=target/readings-$label
rm -rf "$out"
mkdir -p "$out"
N=24
for bits in 3f847ae147ae147b 3ff0000000000000 4010000000000000; do
    pids=()
    for i in $(seq 0 $((N - 1))); do
        nice -n 10 "$exe" --shard "$i/$N" --scale-bits "$bits" > "$out/$bits.$i" &
        pids+=($!)
    done
    for pid in "${pids[@]}"; do
        wait "$pid"
    done
    cat "$out/$bits".[0-9]* > "$out/$bits.all"
    grep '^cell' "$out/$bits.all" | sort > "$out/$bits.cells"
    echo "scale $bits: $(wc -l < "$out/$bits.cells") cells"
done
echo "dump-readings $label: done"
