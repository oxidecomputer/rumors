# rumors workspace — every artifact, tiered by feedback speed.
#
#   inner loop   just check / just test <filter>     seconds-to-a-minute
#   commit gate  just gate                           fmt → docs lint → clippy → docs → tests
#   no-rot sweep just all                            everything below, cheap-first
#
# `all` is the superset: it adds the artifacts the gate doesn't reach — the
# `before` feature matrix, the wasm target, the viz TypeScript bundle, the
# nightly fuzz targets, the bench/example builds, and the formal tier
# (the Lean proofs and the eventdag oracle).

set shell := ["bash", "-euo", "pipefail", "-c"]

# Merged doctests and the fuzz workspace's libFuzzer build need nightly.

nightly_toolchain := "nightly"

# Default fuzz smoke duration per target, in seconds (matches the guidance in
# crates/before/fuzz/Cargo.toml).

fuzz_smoke_secs := "20"

# List recipes.
default:
    @just --list --unsorted

# ── inner loop ───────────────────────────────────────────────────────────────

# Type-check every host target: libs, tests, benches, examples.
# Default features only, so the inner loop skips the `protocol-v1` towers;
# the gate's clippy/docs/test-all still build every feature.
check:
    cargo check --workspace --all-targets

# Codegen-running recipes go through tools/memwatch: a runaway rustc (e.g. a
# monomorphization bomb — see src/tree/traverse/act.rs) or a runaway test
# fails the build with the offender named instead of wedging the machine.
# `check`/`clippy` skip codegen, so they can't detonate one and run bare.
# Override the limits per-invocation: `PROC_LIMIT_GB=16 just test`.

# Run the test suites; pass a filter to narrow (`just test mirror`).
# Default features only: the V1 wire tests (and the V1 towers in every test
# binary) build only in `test-all`, which the gate runs.
test *args:
    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace {{ args }}

# The gate's test run: every feature, including the `protocol-v1` wire tests.
test-all *args:
    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace --all-features {{ args }}

# Doctests — nextest does not run these, so they need their own invocation.
# Stable rustdoc compiles one executable per example; `before` has nearly 100,
# and their macOS link work dominates the gate. Nightly's merged mode compiles
# one harness per crate instead. Keep its target separate so switching compilers
# cannot invalidate the stable gate artifacts (or vice versa).
doctest:
    RUSTDOCFLAGS="-Z unstable-options --merge-doctests yes" {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} test --workspace --doc --all-features --target-dir target/doctest-nightly

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

# rustdoc renders a doc comment's first paragraph as the item's summary — the
# one-liner shown in module index tables and search. tools/doclint fails the
# gate when that paragraph grows past a one-liner, the same trees the audited
# rustdoc covers (before's library and rumors). It needs no build, so it runs
# first for fast failure.

# Flag doc-comment summaries that have outgrown a one-liner.
doclint:
    ./tools/doclint crates/before/src src

# Require every Rust test to document the behavior and invariant it protects.
testdoc:
    ./tools/testdoc --self-test
    ./tools/testdoc .

# tools/readme mirrors each crate's crate-level rustdoc into its README via
# cargo-rdme, then strips the intra-doc links cargo-rdme can't resolve (the
# public types are re-exported from private submodules, and the docs use
# rustdoc's shortcut link form) down to plain code spans. `readme-check`
# re-derives the READMEs into scratch copies and diffs — the same no-rot
# contract as fmt-check, so a rustdoc edit can't silently desync the README.
# Needs cargo-rdme: `cargo install cargo-rdme`.

# Regenerate every crate's README from its crate-level rustdoc.
readme:
    ./tools/readme write

# Verify every README is in sync with its rustdoc (the gate variant of `readme`).
readme-check:
    ./tools/readme check

# Run the pre-commit gate.
gate: fmt-check doclint testdoc readme-check clippy docs docs-internal test-all doctest

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
    cargo check -p rumors --features protocol-v1

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
    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build

# The decode invariant (accepted input re-encodes stably and decodes back to
# itself) is asserted inline in the targets, so any hit is a crash.

# Short fuzz smoke: run each libFuzzer target for `secs` seconds.
[working-directory("crates/before/fuzz")]
fuzz secs=fuzz_smoke_secs:
    cargo +{{ nightly_toolchain }} fuzz run fuzz_decode -- -max_total_time={{ secs }}
    cargo +{{ nightly_toolchain }} fuzz run fuzz_decode_ops -- -max_total_time={{ secs }}

# ── the formal tier (formal/lean; needs elan) ────────────────────────────────
# The proofs are kernel-checked by `lake build` (pins, negative controls,
# invariant preservation); `eventdag` is the progress-lemma oracle and
# schedule-candidate gate (formal/PROGRESS.md §3/§5): DAG acyclicity, totals
# cross-checks, greedy/candidate linearization, replay of the candidate as a
# real model run, a random-skeleton fuzz sweep, and self-testing negative
# controls. Both are local-only: the CI runner has no Lean toolchain.

# Kernel-check the Lean theorem artifact (all proofs, pins, controls).
[working-directory("formal/lean")]
lean:
    PATH="$HOME/.elan/bin:$PATH" lake build

# Run the event-DAG oracle + schedule gate; override seeds: `just eventdag 300`.
[working-directory("formal/lean")]
eventdag fuzz_seeds="100":
    PATH="$HOME/.elan/bin:$PATH" lake build eventdag
    PATH="$HOME/.elan/bin:$PATH" lake exe eventdag eventdag-out {{ fuzz_seeds }}

# ── conveniences ─────────────────────────────────────────────────────────────

# Run benches, e.g. `just bench -p before party` or `just bench gossip_grid`.
bench *args:
    cargo bench {{ args }}

# Run the chatroom demo, e.g. `just rumormill --name alice` (paste a peer id

# into the dialog) or `just rumormill --name bob --peer <endpoint-id>`.
rumormill *args:
    cargo run --release -p rumormill -- {{ args }}

# ── the no-rot sweep ─────────────────────────────────────────────────────────
# `ci` is the build-everything tier: the gate's checks plus the feature matrix,
# wasm, bench builds, the fuzz-target *build*, and the viz bundle. It is ordered
# cheap-first so failures surface early — formatting, then the lint (which also
# compiles all host targets), the feature matrix, wasm, docs, the full
# test+doctest run, bench builds, the fuzz build, and finally the
# network-touching viz bundle. GitHub CI runs exactly this.
#
# `all` is `ci` plus what CI cannot run: a short libFuzzer smoke (poor
# per-commit spend) and the formal tier (the runner has no Lean toolchain) —
# the kernel-checked proofs and the eventdag oracle/schedule gate.

# Build everything (no fuzz run): the no-rot sweep as CI runs it.
ci: fmt-check doclint testdoc readme-check clippy features wasm-check docs docs-internal test-all doctest bench-build fuzz-build viz

# Everything: the no-rot sweep, plus the fuzz smoke and the formal tier.
all: ci (fuzz fuzz_smoke_secs) lean eventdag
