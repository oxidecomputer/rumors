#!/bin/bash
# Surveyor run 1: the five cost-only bits survivors against the fuzz-fit fuel bands.
set -u
ROOT=$(pwd)
OUT=$ROOT/target/survey
mkdir -p "$OUT"
GUEST=$ROOT/target/fuzzfit/wasm32-unknown-unknown/release/fuzzfit_guest.wasm
for v in base packed splice words_none words_lt; do
  if [ "$v" = base ]; then unset SURVEY_MUTANT; else export SURVEY_MUTANT=$v; fi
  echo "=== variant $v (SURVEY_MUTANT=${SURVEY_MUTANT-unset})"
  ( cd crates/before/fuzzfit && nice -n 10 cargo build --locked -p fuzzfit-guest --release --target wasm32-unknown-unknown --target-dir "$ROOT/target/fuzzfit" ) > "$OUT/$v-build.log" 2>&1; echo "guest build $v status $?"
  ( cd crates/before/fuzzfit && nice -n 10 cargo build --locked -p fuzzfit-harness --tests --release ) >> "$OUT/$v-build.log" 2>&1; echo "harness build $v status $?"
  grep -c "Compiling before" "$OUT/$v-build.log" | sed "s/^/compiled-before-count $v: /"
  sha256sum "$GUEST" | sed "s/^/guest-sha $v: /"
  ( cd crates/before/fuzzfit && FUZZFIT_GUEST_WASM="$GUEST" nice -n 10 cargo nextest run --locked --cargo-profile release --no-fail-fast ) > "$OUT/$v-test.log" 2>&1; echo "nextest $v status $?"
  grep -E "^\s+(PASS|FAIL|SIGABRT|TIMEOUT)|Summary" "$OUT/$v-test.log" | sed "s/^/$v: /"
done
echo "=== done"
