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
    cargo clippy -p suanpan -- -D warnings
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
# commit), then fuelscape's sampler pins and pipeline smoke (which reuse
# the guest fuzzfit just built), then the board's cross-process
# determinism tripwire and the sharded-render byte-identity pin, and
# last the surface-totality leg (before's public surface, parsed from
# nightly rustdoc JSON, held total against the operation roster).

# Run the pre-commit gate; it must come up fully clean before every commit.
gate: fmt-check doclint testdoc readme-check clippy clippy-default docs docs-internal test-all doctest fuzzfit-build fuzzfit fuelscape-test amp-board-determinism amp-board-shard-pin worst-cases-pin surface-totality

# ── artifacts the gate doesn't reach ─────────────────────────────────────────
# `borsh` is exercised constantly via rumors; `serde` and `oracle` are only
# ever lit here. The `serde`+`borsh` pair matters because both derive on the
# same types.

# Feature matrix: every cfg-gated surface on its own, so nothing rots behind `--all-features`.
features:
    cargo check -p suanpan --no-default-features
    cargo check -p suanpan --no-default-features --features touch-meter
    cargo check -p before --no-default-features
    cargo check -p before --no-default-features --features serde
    cargo check -p before --no-default-features --features borsh
    cargo check -p before --no-default-features --features oracle
    cargo check -p before --no-default-features --features meter
    cargo check -p before --no-default-features --features laws
    cargo check -p before --no-default-features --features limb-meter
    cargo check -p before --no-default-features --features scan-meter
    cargo check -p before --no-default-features --features serde,borsh
    cargo check -p rumors --no-default-features
    cargo check -p rumors --features protocol-v1
    cargo check -p rumors --features meter

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
# (48 fuzzed programs against the pinned bands, point and shape legs,
# plus the whole 256-program deterministic prefix judged step by step:
# the random draws probe novelty, the prefix leg is total). A failure
# shrinks to a minimal out-of-band shape and writes a proptest seed
# file — commit any seed that appears.

# Run the fuzz-fit asymptotics suites against the pinned fuel bands.
[working-directory("crates/before/fuzzfit")]
fuzzfit: fuzzfit-build
    {{ justfile_directory() }}/tools/memwatch cargo nextest run --cargo-profile release

# Re-fit the pinned bands from the committed deterministic corpus (4096
# programs; byte-reproducible, so any diff is a real change). Rewrites
# harness/src/bands.rs atomically: review the diff like a snapshot and
# commit with a dated movement annotation in the module doc.

# Re-fit and rewrite the fuzz-fit harness's pinned fuel bands.
[working-directory("crates/before/fuzzfit")]
fuzzfit-calibrate: fuzzfit-build
    cargo run --release -p fuzzfit-harness --bin calibrate

# The population atlas lives in its own detached workspace
# (crates/before-fuelscape, the fuzz-fit idiom: workspace-wide builds never
# compile it, and its wasmtime/plotters tooling stays out of the
# production crates' graph); the gate reaches it only through these
# recipes by name. Its committed tests are the sampler adequacy pins —
# counting tables against exhaustive grammar enumeration and the real
# decoders' accept sets, chi-square uniformity, codec round-trips — plus
# a tiny end-to-end pipeline smoke (sample, measure fuel in the fuzz-fit
# guest, render), which is why the guest builds first. Audit-only by
# design: nothing here enforces a fuel number — the envelope suite and
# the fuzz-fit bands own enforcement.

# Lint and test the fuelscape: sampler adequacy pins plus the pipeline smoke.
[working-directory("crates/before-fuelscape")]
fuelscape-test: fuzzfit-build
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    FUZZFIT_GUEST_WASM={{ justfile_directory() }}/target/fuzzfit/wasm32-unknown-unknown/release/fuzzfit_guest.wasm {{ justfile_directory() }}/tools/memwatch cargo nextest run

# Renders one log-log heatmap per public operation into target/fuelscape
# (SVG per op plus a gallery index.html): p(fuel | size) from uniform
# draws over each exact-byte-size canonical input space, the committed
# adversarial families overlaid as marked points, wasmtime instruction
# fuel as the work currency. Deterministic per (seed, plan): re-running
# the same plan on the same guest reproduces every reading. Defaults:
# 300 samples/column to 256 bytes; override e.g.
# `just fuelscape --samples 500 --max-bytes 512`.

# Render the full population atlas into target/fuelscape (audit view; not enforcement).
[working-directory("crates/before-fuelscape")]
fuelscape *args: fuzzfit-build
    FUELSCAPE_TIP=$(git rev-parse HEAD) FUZZFIT_GUEST_WASM={{ justfile_directory() }}/target/fuzzfit/wasm32-unknown-unknown/release/fuzzfit_guest.wasm cargo run --release --bin fuelscape -- --out {{ justfile_directory() }}/target/fuelscape {{ args }}

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

# Full sampling is required for any quoted number. The filter matches
# criterion IDs (`group/function`), so one
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
# One parameterized recipe names both judge axes at the call site — no
# implicit default hides in a bare name. `sampling`: "quick" (10 samples x
# 1 s; the iteration default — like `bench-quick`, never quoted) or
# "record" (criterion's full sampling, required for any quoted number).
# `cells`: "pinned" (the rule-derived subset: each shape's designed-stress
# pairings, the organic control, and the declared-model riders) or "full"
# (the whole shape x operation product; final verdicts, slow — expect full
# runs only at acceptance points, at record sampling). Each run benches the
# board at the default scale and the acceptance scale (x4), saves a
# criterion baseline and a stamped denominator sidecar per scale (the
# stamp binds sidecar to run: scale, profile, sampling, git tip — the
# judge refuses mismatched pairs), and tools/benchjudge fits every cell's
# exponent across the two (denominated against the board's own per-cell
# bytes) at the cell's own ceiling — general 1.3 for the board rows, text
# 1.7 for the wide-display pair, the class declared per cell by the bench
# sidecar, never by the roster — red/green table. Every run judges through
# the committed roster (tools/benchjudge-expected.json: expected reds by
# cell name — the permanent schoolbook tripwire, required RED at its text
# ceiling, plus the hugeleaf display pair — and boundary cells by name,
# that set empty at this tip; tests/bench_judge_roster.rs pins the exact
# membership; the sampling pin covers both modes — the expectations are
# exponent classes, which hold under either regime), so it passes on the
# honest tree while the rostered reds stand and fails on any unexpected
# red OR unexpected green.

# Judge the board bench exponents across both scales through the roster.
bench-judge sampling="quick" cells="pinned":
    @case "{{ sampling }}:{{ cells }}" in (quick:pinned|quick:full|record:pinned|record:full) ;; (*) echo 'bench-judge: sampling is "quick"|"record", cells is "pinned"|"full"' >&2; exit 2;; esac
    ./tools/benchjudge --self-test
    {{ if cells == "full" { "BOARD_BENCH_MODE=full" } else { "" } }} BOARD_BENCH_TIP=$(git rev-parse HEAD) BOARD_BENCH_SCALE=1 BOARD_BENCH_DENOMS={{ criterion_dir }}/board-denoms-lo.json cargo bench -p before --bench board -- {{ if sampling == "quick" { "--sample-size 10 --measurement-time 1" } else { "" } }} --save-baseline board-judge-lo
    {{ if cells == "full" { "BOARD_BENCH_MODE=full" } else { "" } }} BOARD_BENCH_TIP=$(git rev-parse HEAD) BOARD_BENCH_SCALE=acceptance BOARD_BENCH_DENOMS={{ criterion_dir }}/board-denoms-hi.json cargo bench -p before --bench board -- {{ if sampling == "quick" { "--sample-size 10 --measurement-time 1" } else { "" } }} --save-baseline board-judge-hi
    ./tools/benchjudge --criterion-dir {{ criterion_dir }} --lo board-judge-lo --hi board-judge-hi --denoms-lo {{ criterion_dir }}/board-denoms-lo.json --denoms-hi {{ criterion_dir }}/board-denoms-hi.json --tip $(git rev-parse HEAD) --roster {{ benchjudge_roster }}

# An unmetered machine-word quadratic (green on every board counter column)
# must read RED through the judge; the recipe succeeds exactly when it does.
# The same measured shape is pinned in tools/benchjudge --self-test, which
# every bench-judge recipe runs first.

# The judge's live tripwire: a known quadratic must read red, or the sweep fails.
bench-judge-tripwire:
    ./tools/benchjudge --self-test
    BOARD_BENCH_TIP=$(git rev-parse HEAD) BOARD_BENCH_SCALE=1 BOARD_BENCH_DENOMS={{ criterion_dir }}/tripwire-denoms-lo.json cargo bench -p before --bench tripwire -- --sample-size 10 --measurement-time 1 --save-baseline tripwire-judge-lo
    BOARD_BENCH_TIP=$(git rev-parse HEAD) BOARD_BENCH_SCALE=acceptance BOARD_BENCH_DENOMS={{ criterion_dir }}/tripwire-denoms-hi.json cargo bench -p before --bench tripwire -- --sample-size 10 --measurement-time 1 --save-baseline tripwire-judge-hi
    ./tools/benchjudge --expect-red --criterion-dir {{ criterion_dir }} --lo tripwire-judge-lo --hi tripwire-judge-hi --denoms-lo {{ criterion_dir }}/tripwire-denoms-lo.json --denoms-hi {{ criterion_dir }}/tripwire-denoms-hi.json --tip $(git rev-parse HEAD)

# Each board cell judges deterministic work counters (limbs, scans,
# segments, heap) against a pinned proportionality envelope: green means
# work scaled with the input, red is an amplification finding. The board
# reads no clock, so its output is byte-identical under any machine load
# (the time leg lives in bench-judge). Optional scale multiplies the input
# sizes, e.g. `just amp-board 4`.
#
# The board runs at the release profile, the profile of record: debug
# assertions perform metered work (Base comparisons through the limb shim,
# metered probe cursors), so a dev board measures algorithm plus
# verification scaffolding while release measures the production work
# alone. A dev run (`cargo run -p before --example amp_board ...`) remains
# a legitimate debugging view; its numbers must never be pinned anywhere.
#
# Every mode parallelizes by process sharding: the peak-heap column reads
# the process-global allocator, so the sweep stays single-threaded inside
# each process and the runner spawns children that split the operation x
# family cell grid instead, merging the measured samples back in board
# order. Sharding must not move a reading (the amp-board-shard-pin recipe
# holds the sharded render byte-identical to the serial one);
# AMP_BOARD_SHARDS overrides the shard count, and AMP_BOARD_SHARDS=1 is
# the direct in-process serial path.

# Run the amplification board: the red-green resource-proportionality matrix over before's public operations.
amp-board *args:
    cargo run --release -p before --example amp_board --features limb-meter,scan-meter -- {{ args }}

# Acceptance is all green at BOTH the default scale and the acceptance
# scale (board::ACCEPTANCE_SCALE, the segment-onset witness scale), one run
# each: the determinism tripwire (the runner's in-process double
# measurement of every cell, plus this file's cross-process byte-compare)
# is what proves a reading is reproducible, so acceptance needs no repeated
# hand runs.

# Run the amplification board at the acceptance scale.
amp-board-acceptance:
    cargo run --release -p before --example amp_board --features limb-meter,scan-meter -- acceptance

# Every quantity the board judges or renders is a deterministic counter, so
# two whole renders from two processes must be byte-identical under any
# machine load; any diff is a nondeterminism bug in a meter or a measured
# body. This is the cross-process leg of the board's determinism tripwire
# (the in-process leg is the runner itself, which measures every cell twice
# and panics on any counter disagreement, at every scale on every run, in
# every shard child). Both renders take the default sharded path, so the
# comparison also holds the shard spawn/merge pipeline reproducible. The
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

# Process sharding must not move a single reading: the sharded render (the
# default path: child processes splitting the operation x family cell
# grid, each owning its own global allocator) is byte-compared against
# the serial in-process render (AMP_BOARD_SHARDS=1, the reference path)
# at both scales of record. Any diff is a finding to investigate — a
# reading that depends on which
# process measured it (one-time lazy initialization is the known genre) —
# never an accepted delta. On a single-core machine the default path is
# already serial and the comparison is vacuous; the machines of record are
# multi-core. Runs at release, the board's profile of record.

# Byte-compare the sharded board render against the serial reference at both scales of record.
amp-board-shard-pin:
    #!/usr/bin/env bash
    set -euo pipefail
    a=$(mktemp) && b=$(mktemp)
    trap 'rm -f "$a" "$b"' EXIT
    for scale in 1 acceptance; do
        AMP_BOARD_SHARDS=1 cargo run -q --release -p before --example amp_board --features limb-meter,scan-meter -- "$scale" > "$a"
        cargo run -q --release -p before --example amp_board --features limb-meter,scan-meter -- "$scale" > "$b"
        cmp "$a" "$b"
    done

# The surface-totality leg: the operation roster in
# crates/before/src/surface.rs (METHOD_SURFACE, the machine-readable
# enumeration the surface-coverage suite enforces) is held total against
# nightly rustdoc JSON — the compiler's own account of the public
# surface — so a public fn or method added anywhere (a new file, a new
# module, a feature-gated tree) fails the gate until it gains a roster
# row or a named, dated exception in crates/before/surfacecheck. The
# in-tree roster test scans a hand-named source-file list; this leg is
# the other jaw of the pincer, with no file list to forget. The checker
# lives in a detached workspace (the fuzzfit idiom), so ordinary
# workspace builds never compile it; this recipe also runs its lints and
# unit tests, which the workspace-wide gate legs cannot reach.
#
# Toolchain coupling: rustdoc JSON is an unstable format, versioned by
# its `format_version` field, and the checker's `rustdoc-types`
# dependency is pinned exact to the release speaking the installed
# nightly's format. The checker refuses — loudly, naming both numbers —
# any document whose format_version differs, so a nightly bump can fail
# this recipe but can never make it silently wrong. After bumping the
# nightly: run this recipe, and if it reports a format mismatch, move
# the `rustdoc-types` pin in crates/before/surfacecheck/Cargo.toml to
# the release whose FORMAT_VERSION matches the new nightly's output
# (the rustdoc-types changelog maps releases to formats), then re-run
# until green.

# Build the nightly rustdoc JSON the surface-totality check parses.
surface-json:
    cargo +{{ nightly_toolchain }} rustdoc -p before --lib --all-features --target-dir target/surface-json -- -Z unstable-options --output-format json

# Hold before's public surface (from rustdoc JSON) total against the roster.
[working-directory("crates/before/surfacecheck")]
surface-totality: surface-json
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    {{ justfile_directory() }}/tools/memwatch cargo nextest run
    cargo run -q -- {{ justfile_directory() }}/target/surface-json/doc/before.json

# The worst-case map answers "which committed shape is worst for operation
# X" mechanically: for every operation x currency it takes the argmax over
# the family roster of the board's own normalized constants (each cell's
# reading over its own denominator of record), with the runner-up and the
# margin beside it. Honest scope: the maximum over the committed roster --
# the claim that this is the true worst case is carried by the complexity
# claims and their tripwires, not by this table. Runs at release, the
# board's profile of record, at both scales of record (default and
# acceptance), one table each.

# Render the worst-case map: the argmax family per operation x currency, both scales of record.
worst-cases:
    cargo run --release -p before --example amp_board --features limb-meter,scan-meter -- worst-cases

# The map's rankings are pinned: a committed expectation table (the
# WORST_RANKINGS const beside the fold) is entry-compared against the live
# fold at both scales of record, so a ranking flip is caught in the gate,
# never discovered by a reader. A flip is news: either a family
# legitimately overtook (re-pin deliberately with a movement annotation) or
# a code change made some shape relatively worse (investigate first).
# Exits nonzero on any drift, naming the operation, currency, scale, and
# both worsts. Runs at release, the board's profile of record: rankings
# derive from readings, and dev readings are never pinned.

# Entry-compare the live worst-case fold against the committed ranking pin.
worst-cases-pin:
    cargo run --release -p before --example amp_board --features limb-meter,scan-meter -- worst-cases-check

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
