#!/bin/bash
# Scope B: viability sample over the two exploded tuple constructors
# (Work<B>::initiator_level, Work<B>::responder_level) that scope A
# excluded. Their ~45k replacement mutants are the Cartesian product of
# candidate constructions over 5-tuples whose elements (channel halves,
# oneshot::Receiver, Pin<Box<dyn Responses>>, BoxFuture) have no
# construction cargo-mutants can synthesize, so every combination is
# expected UNVIABLE (build failure). Two shards of 512 sample ~176 of
# them from different regions of the list to verify that empirically.
#
# --baseline skip: scope A's baseline already proved the unmutated tree
# builds and its suites pass under the configuration of record; this
# run only classifies viability. --timeout is required with a skipped
# baseline; generous, since no mutant is expected to reach the test
# phase.
set -euo pipefail
unset CARGO_TARGET_DIR
export NEXTEST_TEST_THREADS=8
export CARGO_BUILD_JOBS=8
# cargo-mutants exits nonzero when any mutant is missed or unviable;
# capture the code per shard instead of aborting the loop on it.
for K in 0 257; do
  OUT="$HOME/build/rumors-mutants/scopeB-shard$K"
  mkdir -p "$OUT"
  rc=0
  cargo mutants \
    --file 'src/tree/mirror/streaming/materialized/work/levels.rs' \
    --re 'replace Work<B>::(responder|initiator)_level ->' \
    --shard "$K/512" \
    --baseline skip \
    --timeout 900 \
    --jobs 8 \
    --output "$OUT" \
    --colors never || rc=$?
  echo "scopeB shard $K/512 exit code: $rc"
done
