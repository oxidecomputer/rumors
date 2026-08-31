#!/bin/bash
# Scope A of the rumors CBOR-wire mutation campaign: the six --file
# scopes of the brief, minus the replacement mutants of the two
# tuple-returning level constructors (Work<B>::initiator_level,
# Work<B>::responder_level), whose candidate-construction Cartesian
# product is ~45k mutants; their viability is sampled separately by
# shard (scope B). Runs under the campaign configuration of record in
# .cargo/mutants.toml (nextest, --all-features, dev profile).
#
# CARGO_TARGET_DIR is unset deliberately: the wrapper exports a single
# shared target dir, which would serialize and cross-contaminate the
# per-job scratch builds; each scratch copy builds its own ./target.
set -euo pipefail
unset CARGO_TARGET_DIR
export NEXTEST_TEST_THREADS=8
export CARGO_BUILD_JOBS=8
OUT="$HOME/build/rumors-mutants/scopeA"
mkdir -p "$OUT"
exec cargo mutants \
  --file 'src/tree/mirror/**' \
  --file 'src/bookmark/**' \
  --file 'src/observe.rs' \
  --file 'src/message.rs' \
  --file 'src/batch.rs' \
  --file 'src/peer.rs' \
  --exclude-re 'replace Work<B>::(responder|initiator)_level ->' \
  --jobs 16 \
  --output "$OUT" \
  --colors never
