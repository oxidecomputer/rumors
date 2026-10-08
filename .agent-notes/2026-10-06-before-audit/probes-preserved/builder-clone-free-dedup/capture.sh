#!/bin/bash
# usage: capture.sh <binary> <label>
# Writes ~/src/rumors-slot-23/target/bed/<label>-<scalebits>.cells: every board cell line, sorted,
# at the three scales the board samples (0.01, 1.0, 4.0).
set -u
bin=$1; label=$2; N=24; fail=0
cd ~/src/rumors-slot-23
for scale in 3f847ae147ae147b 3ff0000000000000 4010000000000000; do
  pids=()
  for i in $(seq 0 $((N-1))); do
    nice -n 10 "$bin" --shard $i/$N --scale-bits $scale > target/bed/$label-$scale-$i.raw &
    pids+=($!)
  done
  for p in "${pids[@]}"; do wait $p || { echo "child $p failed at $scale"; fail=1; }; done
  cat target/bed/$label-$scale-*.raw | grep '^cell' | sort > target/bed/$label-$scale.cells
  echo "$label $scale cells=$(wc -l < target/bed/$label-$scale.cells)"
done
exit $fail
