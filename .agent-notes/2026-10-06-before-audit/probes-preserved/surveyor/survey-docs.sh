#!/bin/bash
# Surveyor run 2: does rustdoc with warnings denied pass in the in-scope
# detached workspaces, which no gate or landing-check leg documents?
set -u
ROOT=$(pwd)
mkdir -p "$ROOT/target/survey"
for ws in crates/before/wasm32-pins crates/before/surfacecheck; do
  name=$(basename "$ws")
  for mode in public private; do
    flags=""
    [ "$mode" = private ] && flags="--document-private-items"
    log="$ROOT/target/survey/doc-$name-$mode.log"
    ( cd "$ws" && RUSTDOCFLAGS="-D warnings" nice -n 10 cargo doc --locked --workspace --no-deps $flags --target-dir "$ROOT/target/survey-doc/$name-$mode" ) > "$log" 2>&1
    echo "doc $name $mode status $?"
    grep -E "^(warning|error)" "$log" | sort | uniq -c | head -20
    grep -E -A6 "^error" "$log" | head -60
  done
done
echo "=== done"
