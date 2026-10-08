#!/bin/bash
# Launch one remote matrix job per (build, family); each writes its own log.
S=/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l7
for build in current proto; do
  if [ $build = current ]; then dir=wasm32-pins; else dir=wasm32-pins-proto; fi
  for family in 0 1 2 3 4 5; do
    ( ssh ox-east-1-agent "cd ~/src/rumors-audit-l7-suanpan/crates/before/wasm32-pins && unset CARGO_TARGET_DIR && L7_FAMILIES=$family WASM32_PINS_GUEST_WASM=\$HOME/src/rumors-audit-l7-suanpan/target/$dir/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm nice -n 10 cargo test --locked --release -p wasm32-pins-harness --test pins -- --ignored --nocapture --test-threads 1 zz_l7_fuel_matrix" > $S/m2/$build-f$family.log 2>&1; echo "exit=$?" >> $S/m2/$build-f$family.log ) &
  done
done
wait
echo all-done > $S/m2/ALLDONE
