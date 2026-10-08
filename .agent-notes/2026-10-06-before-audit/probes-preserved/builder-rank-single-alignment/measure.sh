# Usage: measure.sh <label>; run from the synced worktree on the box.
set -e
label=$1
out=target/meas-$label
rm -rf "$out"; mkdir -p "$out"
export AMP_BOARD_SHARDS=24
git rev-parse HEAD > "$out/head"
git status --short > "$out/status"
nice -n 10 cargo run --locked --release -p before --example amp_board --features touch-meter,scan-meter,serde,borsh -- acceptance > "$out/board-acceptance.txt" 2>&1 || echo "board acceptance status $?"
nice -n 10 cargo run --locked --release -p before --example amp_board --features touch-meter,scan-meter,serde,borsh -- worst-cases > "$out/worst-cases.txt" 2>&1 || echo "worst-cases status $?"
nice -n 10 just fuzzfit-calibrate > "$out/calibrate.log" 2>&1 || echo "calibrate status $?"
cp crates/before/fuzzfit/harness/src/bands.rs "$out/bands.rs"
git diff --stat -- crates/before/fuzzfit/harness/src/bands.rs > "$out/bands.diffstat" || true
nice -n 10 just wasm32-pins-build > "$out/pins-build.log" 2>&1 || echo "pins build status $?"
( cd crates/before/wasm32-pins && WASM32_PINS_GUEST_WASM=$PWD/../../../target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm nice -n 10 cargo nextest run --locked --cargo-profile release --no-capture -E 'test(rank_arithmetic)' ) > "$out/pins-rank.log" 2>&1 || echo "pins rank status $?"
echo "measure $label done"
