# rumors workspace — every artifact, tiered by feedback speed.
#
#   inner loop   just check / just test <filter>     seconds-to-a-minute
#   commit gate  just gate                           fmt → clippy → docs → tests → doctests
#   no-rot sweep just all                            everything below, cheap-first
#
# `all` is the superset: it adds the artifacts the gate doesn't reach — the
# `before` feature matrix, the wasm target, the viz TypeScript bundle, the
# nightly fuzz targets, and the bench/example builds.

set shell := ["bash", "-euo", "pipefail", "-c"]

# The fuzz workspace needs libFuzzer, hence nightly.

fuzz_toolchain := "nightly"

# Default fuzz smoke duration per target, in seconds (matches the guidance in
# crates/before/fuzz/Cargo.toml).

fuzz_smoke_secs := "20"

# List recipes.
default:
    @just --list --unsorted

# ── inner loop ───────────────────────────────────────────────────────────────

# Type-check every host target: libs, tests, benches, examples.
check:
    cargo check --workspace --all-targets --all-features

# Codegen-running recipes go through tools/memwatch: a runaway rustc (e.g. a
# monomorphization bomb — see src/tree/traverse/act.rs) or a runaway test
# fails the build with the offender named instead of wedging the machine.
# `check`/`clippy` skip codegen, so they can't detonate one and run bare.
# Override the limits per-invocation: `PROC_LIMIT_GB=16 just test`.

# Run the test suites; pass a filter to narrow (`just test mirror`).
test *args:
    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace --all-features {{ args }}

# Doctests — nextest does not run these, so they need their own invocation.
doctest:
    {{ justfile_directory() }}/tools/memwatch cargo test --workspace --doc --all-features

# Lint every target, warnings denied (the commit-gate setting).
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format the whole workspace.
fmt:
    cargo fmt --all

# Verify formatting without rewriting (the gate variant of `fmt`).
fmt-check:
    cargo fmt --all --check

# ── commit gate (CLAUDE.md: fmt → clippy → docs → test, all clean) ───────────

# Run the pre-commit gate.
gate: fmt-check clippy docs docs-internal test doctest

# ── artifacts the gate doesn't reach ─────────────────────────────────────────
# `borsh` is exercised constantly via rumors; `serde` and `oracle` are only
# ever lit here. The `serde`+`borsh` pair matters because both derive on the
# same types.

# Feature matrix: every cfg-gated surface on its own, so nothing rots behind `--all-features`.
features:
    cargo check -p before --no-default-features
    cargo check -p before --no-default-features --features serde
    cargo check -p before --no-default-features --features borsh
    cargo check -p before --no-default-features --features oracle
    cargo check -p before --no-default-features --features serde,borsh
    cargo check -p rumors --no-default-features

# The viz engine must keep compiling for its real target, not just the host.
wasm-check:
    cargo check -p before-viz --target wasm32-unknown-unknown

# This is exactly what the Pages deploy runs. Needs npm (network on first run).

# Full visualizer build: wasm-pack, strict TypeScript typecheck, esbuild bundle.
viz:
    ./crates/before-viz/build.sh

# This catches broken intra-doc links. CLAUDE.md calls the rustdoc the
# documentation of record, so it's load-bearing and part of the gate.

# Build the rustdoc with warnings denied.
docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps

# The public build above never renders private items, so a stale intra-doc
# link inside a private module sails through it. This pass documents private
# items too. It cannot replace `docs`: with private items rendered, the
# `private_intra_doc_links` lint (public docs linking to a private item) no
# longer fires, so each pass catches a class the other cannot. A separate
# target dir keeps the two from invalidating each other's fingerprints, which
# would otherwise re-doc the whole workspace twice on every gate.

# Build the rustdoc including private items, warnings denied.
docs-internal:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --document-private-items --target-dir target/doc-internal

# The only build that exercises the bench/release profile; benches otherwise rot silently.

# Compile (don't run) the criterion benches.
bench-build:
    {{ justfile_directory() }}/tools/memwatch cargo bench --workspace --no-run

# The fuzz targets live in a detached workspace (crates/before/fuzz) precisely
# so the ordinary gate never compiles them: without this recipe they rot invisibly.

# Build the libFuzzer targets (nightly).
[working-directory("crates/before/fuzz")]
fuzz-build:
    {{ justfile_directory() }}/tools/memwatch cargo +{{ fuzz_toolchain }} fuzz build

# The decode invariant (accepted input re-encodes stably and decodes back to
# itself) is asserted inline in the targets, so any hit is a crash.

# Short fuzz smoke: run each libFuzzer target for `secs` seconds.
[working-directory("crates/before/fuzz")]
fuzz secs=fuzz_smoke_secs:
    cargo +{{ fuzz_toolchain }} fuzz run fuzz_decode -- -max_total_time={{ secs }}
    cargo +{{ fuzz_toolchain }} fuzz run fuzz_decode_ops -- -max_total_time={{ secs }}

# ── conveniences ─────────────────────────────────────────────────────────────

# Run benches, e.g. `just bench -p before party` or `just bench gossip_grid`.
bench *args:
    cargo bench {{ args }}

# Run the chatroom demo, e.g. `just rumormill --name alice` (paste a peer id

# into the dialog) or `just rumormill --name bob --peer <endpoint-id>`.
rumormill *args:
    cargo run --release -p rumormill -- {{ args }}

# ── the no-rot sweep ─────────────────────────────────────────────────────────
# Ordered cheap-first so failures surface early: formatting, then the lint
# (which also compiles all host targets), the feature matrix, wasm, docs, the
# full test+doctest run, bench builds, fuzz build + smoke, and finally the
# network-touching viz bundle.

# Everything: the full no-rot sweep.
all: fmt-check clippy features wasm-check docs docs-internal test doctest bench-build fuzz-build (fuzz fuzz_smoke_secs) viz
