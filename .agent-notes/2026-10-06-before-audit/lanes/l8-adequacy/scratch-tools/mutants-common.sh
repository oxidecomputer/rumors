# Sourced on the box by every mutation invocation: one place for the settings
# every run shares, so pilot and campaign cannot drift apart.
unset CARGO_TARGET_DIR
export TMPDIR="$HOME/src/rumors-audit-l8-adequacy/target/l8/tmp"
mkdir -p "$TMPDIR"
export PROPTEST_RNG_SEED=8008
export CARGO_PROFILE_DEV_DEBUG=false
export CARGO_PROFILE_TEST_DEBUG=false
MUTANTS_COMMON=(--no-config --test-tool nextest --all-features --baseline skip
  --timeout 400 --build-timeout 900 --jobserver-tasks 24 --cap-lints true
  --exclude "crates/before/src/testing/**" --exclude "crates/before/src/testing.rs"
  --exclude "**/tests.rs" --exclude "**/tests/**")
