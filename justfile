# rumors workspace: the source of truth for verification. Every artifact in
# the workspace has a recipe here, tiered by feedback speed, and `just --list`
# is the tour.
#
#   inner loop   just check / just test <filter>     seconds to a minute
#   commit gate  just gate                           fully clean before every commit
#   no-rot sweep just ci / just all                  everything, so nothing rots
#
# The gate runs every check a commit must pass; its recipe line spells out the
# order. `ci` adds the artifacts the gate doesn't reach, exactly as GitHub CI
# builds them; `all` adds what CI cannot run (the fuzz smoke and the formal
# tier). The comment above each recipe states what it verifies and why.

set shell := ["bash", "-euo", "pipefail", "-c"]

# Merged doctests and the fuzz workspace's libFuzzer build need nightly.

nightly_toolchain := "nightly"

# Default fuzz smoke duration per target, in seconds (matches the guidance in
# crates/before/fuzz/Cargo.toml).

fuzz_smoke_secs := "20"

# Criterion's output root: the bench-judge recipes save baselines and
# denominator sidecars here, honoring CARGO_TARGET_DIR so a fresh or
# redirected target directory keeps the baselines and the sidecars together.

criterion_dir := env_var_or_default("CARGO_TARGET_DIR", justfile_directory() + "/target") + "/criterion"

# The bench judge's committed expected-verdict roster (membership by cell
# name; tools/benchjudge documents the classes and the enforcement).

benchjudge_roster := justfile_directory() + "/tools/benchjudge-expected.json"

# List recipes.
default:
    @just --list --unsorted

# ── inner loop ───────────────────────────────────────────────────────────────

# Default features only, so the inner loop skips the `protocol-v1` towers;
# the gate's clippy/docs/test-all still build every feature.

# Type-check every host target: libs, tests, benches, examples.
check:
    cargo check --workspace --all-targets

# Codegen-running recipes go through tools/memwatch: a runaway rustc (e.g. a
# monomorphization bomb — see src/tree/traverse/act.rs) or a runaway test
# fails the build with the offender named instead of wedging the machine.
# `check`/`clippy` skip codegen, so they can't detonate one and run bare.
# Override the limits per-invocation: `PROC_LIMIT_GB=16 just test`.

# Default features only: the V1 wire tests (and the V1 towers in every test
# binary) build only in `test-all`, which the gate runs.

# Run the test suites; pass a filter to narrow (`just test mirror`).
test *args:
    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace {{ args }}

# The gate's test run: every feature, including the `protocol-v1` wire tests.
test-all *args:
    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace --all-features {{ args }}

# Stable rustdoc compiles one executable per example; `before` has nearly 100,
# and their macOS link work dominates the gate. Nightly's merged mode compiles
# one harness per crate instead. Keep its target separate so switching compilers
# cannot invalidate the stable gate artifacts (or vice versa).

# Run the doctests (nightly), which nextest does not run.
doctest:
    RUSTDOCFLAGS="-Z unstable-options --merge-doctests yes" {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} test --workspace --doc --all-features --target-dir target/doctest-nightly

# Lint every target, warnings denied (the commit-gate setting).
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# `clippy` above lints under --all-features, and the dev-dependency cycle
# forces the meter/oracle features onto the lib for every test build — so a
# surface that is dead under *default* features (test-only helpers left
# ungated) never trips it. These are the default-feature library builds,
# exactly what `cargo build -p <crate>` compiles, warnings denied: test- and
# meter-only surface must be cfg-gated, not left dangling. Each package is
# linted alone so workspace feature unification cannot re-light the gated
# features.

# Lint the default-feature library builds, warnings denied.
clippy-default:
    cargo clippy -p before -- -D warnings
    cargo clippy -p rumors -- -D warnings

# Format the whole workspace.
fmt:
    cargo fmt --all

# Verify formatting without rewriting (the gate variant of `fmt`).
fmt-check:
    cargo fmt --all --check

# ── commit gate: everything a commit must pass, fully clean ──────────────────

# rustdoc renders a doc comment's first paragraph as the item's summary: the
# one-liner shown in module index tables and search. tools/doclint fails the
# gate when that paragraph grows past a one-liner; the fix is to move the
# rest below a blank `///` line. It covers every Rust source in the
# workspace (libraries, tests, benches, examples, the demo crate), and it
# needs no build, so it runs first for fast failure.

# Flag doc-comment summaries that have outgrown a one-liner.
doclint:
    ./tools/doclint benches crates examples src tests

# Require every Rust test to document the behavior and invariant it protects.
testdoc:
    ./tools/testdoc --self-test
    ./tools/testdoc .

# tools/readme mirrors each crate's crate-level rustdoc into its README via
# cargo-rdme, then strips the intra-doc links cargo-rdme can't resolve (the
# public types are re-exported from private submodules, and the docs use
# rustdoc's shortcut link form) down to plain code spans. The READMEs are
# derived, never hand-edited: after editing crate-level rustdoc, run
# `just readme`. `readme-check` re-derives the READMEs into scratch copies
# and diffs, the same no-rot contract as fmt-check, so a rustdoc edit can't
# silently desync the README. Needs cargo-rdme: `cargo install cargo-rdme`.

# Regenerate every crate's README from its crate-level rustdoc.
readme:
    ./tools/readme write

# Verify every README is in sync with its rustdoc (the gate variant of `readme`).
readme-check:
    ./tools/readme check

# The dependency list is the ordering: build-free lints first for fast
# failure, then the builds, then the full-feature tests and doctests, then
# the fuel-band asymptotics check (fuzzfit-build then fuzzfit: the wasm
# guest's per-operation fuel readings judged against the pinned bands, so
# a kernel change that moves fuel fails the commit that carries it — the
# deliberate path is a `just fuzzfit-calibrate` re-pin riding the same
# commit), then the board's cross-process determinism tripwire.

# Run the pre-commit gate; it must come up fully clean before every commit.
gate: fmt-check doclint testdoc readme-check clippy clippy-default docs docs-internal test-all doctest fuzzfit-build fuzzfit amp-board-determinism

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
    cargo check -p before --no-default-features --features meter
    cargo check -p before --no-default-features --features limb-meter
    cargo check -p before --no-default-features --features scan-meter
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

# This catches broken intra-doc links. AGENTS.md calls the rustdoc the
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
# itself) and the `before::laws` law collection are asserted inline in the
# targets, so any hit is a crash. Each run names two corpus directories:
# libFuzzer reads seeds from both and writes new discoveries to the first, so
# the committed `seeds/<target>/` corpus (derived from the live API;
# `tests/fuzz_seeds.rs` gates it) actually seeds every run while staying
# pristine.

# Short fuzz smoke: run each libFuzzer target for `secs` seconds.
[working-directory("crates/before/fuzz")]
fuzz secs=fuzz_smoke_secs:
    cargo +{{ nightly_toolchain }} fuzz run fuzz_decode corpus/fuzz_decode seeds/fuzz_decode -- -max_total_time={{ secs }}
    cargo +{{ nightly_toolchain }} fuzz run fuzz_decode_ops corpus/fuzz_decode_ops seeds/fuzz_decode_ops -- -max_total_time={{ secs }}
    cargo +{{ nightly_toolchain }} fuzz run fuzz_laws corpus/fuzz_laws seeds/fuzz_laws -- -max_total_time={{ secs }}

# The fuzz-fit asymptotics harness lives in a detached workspace
# (crates/before/fuzzfit, the fuzz-target idiom), so workspace-wide builds
# never compile it and wasmtime stays out of the production crates' graph;
# the gate reaches it only through these recipes by name. The guest
# compiles before's public surface to wasm32-unknown-unknown; the harness
# replays fuzzed operation programs natively and under wasmtime fuel
# metering (deterministic instruction counts, byte-reproducible under any
# load) and judges every step against the pinned per-operation fuel bands
# in harness/src/bands.rs — the committed cost law for every public
# operation, so a change that moves an operation's asymptotics fails here
# and re-pins deliberately (`just fuzzfit-calibrate`) instead of drifting.

# Build the fuzz-fit wasm guest and its harness (both halves).
[working-directory("crates/before/fuzzfit")]
fuzzfit-build:
    cargo build -p fuzzfit-guest --release --target wasm32-unknown-unknown
    {{ justfile_directory() }}/tools/memwatch cargo build -p fuzzfit-harness --tests --release

# Run the fuzz-fit suites: generator sanity, meter liveness, the judgment
# and shape-leg tripwires, the quadratic-burner adequacy check, the
# toolchain-pin and staleness cross-checks, and the enforcement sentry
# (48 fuzzed programs against the pinned bands, point and shape legs). A
# failure shrinks to a minimal out-of-band shape and writes a proptest
# seed file — commit any seed that appears.

# Run the fuzz-fit asymptotics suites against the pinned fuel bands.
[working-directory("crates/before/fuzzfit")]
fuzzfit: fuzzfit-build
    {{ justfile_directory() }}/tools/memwatch cargo nextest run --cargo-profile release

# Re-fit the pinned bands from the deterministic corpus of record (1536
# programs; byte-reproducible, so any diff is a real change). Rewrites
# harness/src/bands.rs atomically: review the diff like a snapshot and
# commit with a dated movement annotation in the module doc.

# Re-fit and rewrite the fuzz-fit harness's pinned fuel bands.
[working-directory("crates/before/fuzzfit")]
fuzzfit-calibrate: fuzzfit-build
    cargo run --release -p fuzzfit-harness --bin calibrate

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

# Override seeds UPWARD only (`just eventdag 300`): small seed counts fail
# by design, because the vacuity meta-controls demand enough runs to
# reproduce the known adversarial stalls. The default 100 is a floor, not
# a suggestion.

# Run the event-DAG oracle and schedule gate; the seed count only goes up.
[working-directory("formal/lean")]
eventdag fuzz_seeds="100":
    PATH="$HOME/.elan/bin:$PATH" lake build eventdag
    PATH="$HOME/.elan/bin:$PATH" lake exe eventdag eventdag-out {{ fuzz_seeds }}

# The golden file is formal/lean/muxprobe-expected.tsv: strategy × skeleton
# × capacity × interleaving on the real Mux semantics, plus the
# commit-singleton scan and a random margin-0 sweep. Override seeds:
# `just muxprobe 100`. After a deliberate model or matrix change, regenerate
# the golden inside formal/lean with `lake exe muxprobe --update` and review
# the diff like a snapshot.

# Run the mux executable-evidence matrix against its committed golden file.
[working-directory("formal/lean")]
muxprobe rand_seeds="25":
    PATH="$HOME/.elan/bin:$PATH" lake build muxprobe
    PATH="$HOME/.elan/bin:$PATH" lake exe muxprobe {{ rand_seeds }}

# ── conveniences ─────────────────────────────────────────────────────────────

# Deterministic closed-form arithmetic, no sessions. Written to a temp
# file and moved into place on success, so a failed build cannot truncate
# the tracked table; the window suite byte-compares the committed file
# against the same rendering, so drift fails the gate.

# Regenerate the sync-budget trade-off table compiled into the rustdoc.
window-tradeoff:
    cargo run --example window_tradeoff > src/tree/mirror/streaming/window/tradeoff.md.tmp
    mv src/tree/mirror/streaming/window/tradeoff.md.tmp src/tree/mirror/streaming/window/tradeoff.md

# Full sampling is the mode required for any number quoted as a result of
# record. The filter matches criterion IDs (`group/function`), so one
# operation or one cell is a run: `just bench board version_rank`,
# `just bench board version_rank/harmonic`, `just bench version merge`,
# `just bench gossip_grid`. The board target's IDs mirror the amplification
# board's op x family names cell for cell.

# Run one bench target through a criterion filter, at full sampling.
bench target *filter:
    cargo bench --workspace --bench {{ target }} -- {{ filter }}

# The reduced-sampling inner loop: `bench` at 10 samples x 1 s. Never quoted.
bench-quick target *filter:
    cargo bench --workspace --bench {{ target }} -- --sample-size 10 --measurement-time 1 {{ filter }}

# The amplification board's time leg: the board itself judges deterministic
# counters and floors only (its output is byte-identical under any machine
# load), so the time-exponent judgment runs here, over criterion medians.
# Each recipe runs the board benches at the default scale and the record
# scale (x4), saves a criterion baseline and a stamped denominator sidecar
# per scale (the stamp binds sidecar to run: scale, profile, sampling,
# git tip — the judge refuses mismatched pairs), and tools/benchjudge fits
# every cell's exponent across the two (denominated against the board's own
# per-cell bytes) at the cell's own ceiling — general 1.3 for the board
# rows, text 1.7 for the wide-display pair, the class declared per cell by
# the bench sidecar, never by the roster — red/green table. Both judging
# recipes judge through the committed roster (tools/benchjudge-expected.json:
# expected reds — the 16 bigroot cells awaiting C2 plus the permanent
# schoolbook tripwire, required RED at its text ceiling — and boundary
# cells by name; the bigroot and boundary sets empty at C3), so they pass
# on the honest tree while the owned reds await their cures and fail on
# any unexpected red OR unexpected green.

# Judge the board bench exponents across both scales through the roster (quick mode: iteration only).
bench-judge:
    ./tools/benchjudge --self-test
    BOARD_BENCH_TIP=$(git rev-parse HEAD) BOARD_BENCH_SCALE=1 BOARD_BENCH_DENOMS={{ criterion_dir }}/board-denoms-lo.json cargo bench -p before --bench board -- --sample-size 10 --measurement-time 1 --save-baseline board-judge-lo
    BOARD_BENCH_TIP=$(git rev-parse HEAD) BOARD_BENCH_SCALE=record BOARD_BENCH_DENOMS={{ criterion_dir }}/board-denoms-hi.json cargo bench -p before --bench board -- --sample-size 10 --measurement-time 1 --save-baseline board-judge-hi
    ./tools/benchjudge --criterion-dir {{ criterion_dir }} --lo board-judge-lo --hi board-judge-hi --denoms-lo {{ criterion_dir }}/board-denoms-lo.json --denoms-hi {{ criterion_dir }}/board-denoms-hi.json --tip $(git rev-parse HEAD) --roster {{ benchjudge_roster }}

# Judged through the same roster (its sampling pin covers both modes — the
# expectations are exponent classes, which hold under either regime), so
# the posture is identical in both modes: roster-satisfied on the honest
# tree, the bigroot set emptied at C3, the text expectations permanent.

# `bench-judge` at full sampling: the mode required for numbers of record.
bench-judge-record:
    ./tools/benchjudge --self-test
    BOARD_BENCH_TIP=$(git rev-parse HEAD) BOARD_BENCH_SCALE=1 BOARD_BENCH_DENOMS={{ criterion_dir }}/board-denoms-lo.json cargo bench -p before --bench board -- --save-baseline board-judge-lo
    BOARD_BENCH_TIP=$(git rev-parse HEAD) BOARD_BENCH_SCALE=record BOARD_BENCH_DENOMS={{ criterion_dir }}/board-denoms-hi.json cargo bench -p before --bench board -- --save-baseline board-judge-hi
    ./tools/benchjudge --criterion-dir {{ criterion_dir }} --lo board-judge-lo --hi board-judge-hi --denoms-lo {{ criterion_dir }}/board-denoms-lo.json --denoms-hi {{ criterion_dir }}/board-denoms-hi.json --tip $(git rev-parse HEAD) --roster {{ benchjudge_roster }}

# The bench targets time a BenchMode slice of the board's shape x operation
# product, both modes derived from the board's own axis declarations: the
# default pinned subset (each shape's designed-stress pairings, the organic
# control, and the board-red riders) is what the judging recipes above run;
# BOARD_BENCH_MODE=full times the whole product and is the mode for final
# verdicts. Full-product judge runs pair with the same roster; expect them
# only at acceptance points, at full sampling.

# Judge the WHOLE product's bench exponents across both scales (final verdicts; slow).
bench-judge-full:
    ./tools/benchjudge --self-test
    BOARD_BENCH_MODE=full BOARD_BENCH_TIP=$(git rev-parse HEAD) BOARD_BENCH_SCALE=1 BOARD_BENCH_DENOMS={{ criterion_dir }}/board-denoms-lo.json cargo bench -p before --bench board -- --save-baseline board-judge-lo
    BOARD_BENCH_MODE=full BOARD_BENCH_TIP=$(git rev-parse HEAD) BOARD_BENCH_SCALE=record BOARD_BENCH_DENOMS={{ criterion_dir }}/board-denoms-hi.json cargo bench -p before --bench board -- --save-baseline board-judge-hi
    ./tools/benchjudge --criterion-dir {{ criterion_dir }} --lo board-judge-lo --hi board-judge-hi --denoms-lo {{ criterion_dir }}/board-denoms-lo.json --denoms-hi {{ criterion_dir }}/board-denoms-hi.json --tip $(git rev-parse HEAD) --roster {{ benchjudge_roster }}

# An unmetered machine-word quadratic (green on every board counter column)
# must read RED through the judge; the recipe succeeds exactly when it does.
# The same measured shape is pinned in tools/benchjudge --self-test, which
# every bench-judge recipe runs first.

# The judge's live tripwire: a known quadratic must read red, or the sweep fails.
bench-judge-tripwire:
    ./tools/benchjudge --self-test
    BOARD_BENCH_TIP=$(git rev-parse HEAD) BOARD_BENCH_SCALE=1 BOARD_BENCH_DENOMS={{ criterion_dir }}/tripwire-denoms-lo.json cargo bench -p before --bench tripwire -- --sample-size 10 --measurement-time 1 --save-baseline tripwire-judge-lo
    BOARD_BENCH_TIP=$(git rev-parse HEAD) BOARD_BENCH_SCALE=record BOARD_BENCH_DENOMS={{ criterion_dir }}/tripwire-denoms-hi.json cargo bench -p before --bench tripwire -- --sample-size 10 --measurement-time 1 --save-baseline tripwire-judge-hi
    ./tools/benchjudge --expect-red --criterion-dir {{ criterion_dir }} --lo tripwire-judge-lo --hi tripwire-judge-hi --denoms-lo {{ criterion_dir }}/tripwire-denoms-lo.json --denoms-hi {{ criterion_dir }}/tripwire-denoms-hi.json --tip $(git rev-parse HEAD)

# Each board cell judges deterministic work counters (limbs, scans,
# segments, heap) against a pinned proportionality envelope: green means
# work scaled with the input, red is an amplification finding. The board
# reads no clock, so its output is byte-identical under any machine load
# (the time leg lives in bench-judge). Optional scale multiplies the input
# sizes, e.g. `just amp-board 4`.
#
# The board runs at the release profile, the measurement of record: debug
# assertions perform metered work (Base comparisons through the limb shim,
# metered probe cursors), so a dev board measures algorithm plus
# verification scaffolding while release measures the production work
# alone. A dev run (`cargo run -p before --example amp_board ...`) remains
# a legitimate debugging view; its numbers must never be pinned anywhere.

# Run the amplification board: the red-green resource-proportionality matrix over before's public operations.
amp-board *args:
    cargo run --release -p before --example amp_board --features limb-meter,scan-meter -- {{ args }}

# Acceptance is all green at BOTH the default scale and the record scale
# (board::RECORD_SCALE, the segment-onset witness scale), one run each:
# the determinism tripwire (the runner's in-process double measurement of
# every cell, plus this file's cross-process byte-compare) is what proves a
# reading is reproducible, so acceptance needs no repeated hand runs.

# Run the amplification board at the acceptance scale of record.
amp-board-record:
    cargo run --release -p before --example amp_board --features limb-meter,scan-meter -- record

# Every quantity the board judges or renders is a deterministic counter, so
# two whole renders from two processes must be byte-identical under any
# machine load; any diff is a nondeterminism bug in a meter or a measured
# body. This is the cross-process leg of the board's determinism tripwire
# (the in-process leg is the runner itself, which measures every cell twice
# and panics on any counter disagreement, at every scale on every run). The
# reduced default scale keeps the gate fast; the runner's leg covers the
# acceptance scales. Runs at release, the board's profile of record.

# Byte-compare two cross-process board renders (the determinism gate).
amp-board-determinism scale="0.25":
    #!/usr/bin/env bash
    set -euo pipefail
    a=$(mktemp) && b=$(mktemp)
    trap 'rm -f "$a" "$b"' EXIT
    cargo run -q --release -p before --example amp_board --features limb-meter,scan-meter -- {{ scale }} > "$a"
    cargo run -q --release -p before --example amp_board --features limb-meter,scan-meter -- {{ scale }} > "$b"
    cmp "$a" "$b"

# Paste a peer id into the dialog, or dial one directly:
# `just rumormill --name bob --peer <endpoint-id>`.

# Run the chatroom demo, e.g. `just rumormill --name alice`.
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
# per-commit spend), the formal tier (the runner has no Lean toolchain) —
# the kernel-checked proofs, the eventdag oracle/schedule gate, and the
# muxprobe matrix gate — and the bench judge's two legs: the roster-mode
# judgment (minutes of criterion runs at two scales; quick mode, so its
# exponents are judged but never quoted) and the seconds-scale live
# tripwire, so the judge's red path rides every sweep.

# Build everything (no fuzz run): the no-rot sweep as CI runs it.
ci: fmt-check doclint testdoc readme-check clippy clippy-default features wasm-check docs docs-internal test-all doctest bench-build fuzz-build viz

# Everything: the no-rot sweep, plus the fuzz smoke, the formal tier, and the bench judge.
all: ci (fuzz fuzz_smoke_secs) lean eventdag muxprobe bench-judge bench-judge-tripwire
