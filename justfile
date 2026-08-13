# rumors workspace: the source of truth for verification. Every artifact in
# the workspace has a recipe here, tiered by feedback speed, and `just --list`
# is the tour.
#
#   inner loop   just check / just test <filter>     seconds to a minute
#   commit gate  just gate                           fully clean before every commit
#   no-rot sweep just ci / just all                  everything, so nothing rots
#
# The gate runs every check a commit must pass: build-free lints first,
# then every building leg concurrently (see the comment above `gate` for
# the stream grouping and why parallelism cannot move a verdict).
# `ci` builds the artifacts the gate doesn't reach (the feature matrix, wasm,
# bench builds, the viz bundle), exactly as GitHub CI builds them; `all` adds
# what CI cannot run (the fuzz smoke and the formal tier). Neither sweep
# repeats the gate's instrument legs — the fuel bands, the board verdicts
# and pins, and surface totality run in `just gate`, and GitHub CI's
# `instruments` job re-runs the counter-based subset (the workflow file
# says which legs stay local and why). The comment above each recipe states
# what it verifies and why.

set shell := ["bash", "-euo", "pipefail", "-c"]

# Merged doctests and the fuzz workspace's libFuzzer build need nightly.
#
# Dated, not floating, because one of its outputs is pinned against: the
# surface-totality leg parses rustdoc JSON, whose schema carries a
# `format_version` set by the emitting nightly, and the checker's
# `rustdoc-types` dependency is pinned exact to the release speaking it.
# A floating `nightly` couples a committed pin to whatever each machine
# last downloaded, so the leg passes or fails on the coincidence of two
# rustup states rather than on the tree — and it reads as a portability
# failure on the second machine when it is really a reproducibility one.
# This nightly emits format 59, which is what surfacecheck pins.
#
# Bumping it is deliberate and paired: change the date here, run
# `just surface-totality`, and move the `rustdoc-types` pin to the
# release whose FORMAT_VERSION matches (that recipe's comment carries the
# procedure).

nightly_toolchain := "nightly-2026-06-30"

# The triple the fuzz recipes build for. cargo-fuzz defaults `--target` to the
# triple it was itself built for, not the host's, so a statically linked
# prebuilt (what the CI installer ships) aims the sanitizer build at
# `*-linux-musl`: a target whose std is absent, and whose static libc the
# sanitizer refuses outright. Naming the host keeps the recipes indifferent to
# how cargo-fuzz arrived.

host_triple := `rustc -vV | sed -n 's/^host: //p'`

# Default fuzz smoke duration per target, in seconds (matches the guidance in
# crates/before/fuzz/Cargo.toml).

fuzz_smoke_secs := "20"

# The fuzz-fit guest's wasm: one artifact produced by one recipe and read
# by two others (the fuzz-fit harness and the fuelscape pipeline), in
# separate detached workspaces.
#
# Named here, and passed explicitly at the producer and at every consumer,
# because the location is otherwise a coincidence: the fuzzfit workspace's
# `.cargo/config.toml` points its target dir at the repo root's
# `target/fuzzfit`, and an ambient `CARGO_TARGET_DIR` silently overrides
# that — so the guest is written somewhere else while the consumers keep
# reading the configured path. Anyone who exports `CARGO_TARGET_DIR`, and
# any harness that sets one to keep artifacts outside a synced tree, gets
# a "guest wasm not found" that names a path nothing wrote.

fuzzfit_target := justfile_directory() + "/target/fuzzfit"
fuzzfit_guest_wasm := fuzzfit_target + "/wasm32-unknown-unknown/release/fuzzfit_guest.wasm"

# Criterion's output root: the bench-judge recipes save baselines and
# denominator sidecars here, honoring CARGO_TARGET_DIR so a fresh or
# redirected target directory keeps the baselines and the sidecars together.

criterion_dir := env("CARGO_TARGET_DIR", justfile_directory() + "/target") + "/criterion"

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
# Override the limits per-invocation: `PROC_LIMIT_GB=64 just test`.

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
# ungated) never trips it. These are the default-feature library and
# test-target builds, warnings denied: test- and meter-only surface must be
# cfg-gated, not left dangling, in the integration-test and cfg(test) trees
# just as in the lib (`just test` compiles the test targets under default
# features with warnings not denied, so without this leg that surface never
# meets -D warnings anywhere). Each package is linted alone so workspace
# feature unification cannot re-light the gated features.

# Lint the default-feature library and test builds, warnings denied.
clippy-default:
    cargo clippy -p suanpan --lib --tests -- -D warnings
    cargo clippy -p before --lib --tests -- -D warnings
    cargo clippy -p rumors --lib --tests -- -D warnings

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
# silently desync the README. It leads with the stripper's own self-test, which
# pins the link forms rewritten and the forms preserved: a stripping bug then
# names itself here instead of arriving as unexplained drift in a derived file,
# or as corruption that a regeneration quietly commits.
# Needs cargo-rdme: `cargo install cargo-rdme`.

# Regenerate every crate's README from its crate-level rustdoc.
readme:
    ./tools/readme write

# Verify every README is in sync with its rustdoc (the gate variant of `readme`).
readme-check:
    ./tools/readme self-test
    ./tools/readme check

# This catches broken intra-doc links. AGENTS.md calls the rustdoc the
# documentation of record, so it's load-bearing and part of the gate.
#
# The header flag injects the fuelscape widget assets (the interactive
# measured-growth explorers in before's # Complexity sections) into
# every page's head. RUSTDOCFLAGS is workspace-wide — cargo has no
# per-crate rustdocflags — so non-before pages carry ~40 KB of inert
# head weight; the script activates only on .fuelscape elements.
# docs.rs applies the same flag through before's package metadata.

# Build the rustdoc with warnings denied.
docs:
    RUSTDOCFLAGS="-D warnings --html-in-header {{ justfile_directory() }}/crates/before/docs/fuelscape-header.html" cargo doc --workspace --all-features --no-deps

# The public build above never renders private items, so a stale intra-doc
# link inside a private module sails through it. This pass documents private
# items too. It cannot replace `docs`: with private items rendered, the
# `private_intra_doc_links` lint (public docs linking to a private item) no
# longer fires, so each pass catches a class the other cannot. A separate
# target dir keeps the two from invalidating each other's fingerprints, which
# would otherwise re-doc the whole workspace twice on every gate.

# Build the rustdoc including private items, warnings denied.
docs-internal:
    RUSTDOCFLAGS="-D warnings --html-in-header {{ justfile_directory() }}/crates/before/docs/fuelscape-header.html" cargo doc --workspace --all-features --no-deps --document-private-items --target-dir target/doc-internal

# The before coverage roster (crates/before/src/surface.rs) and the bespoke
# half of the pointwise-differential tiling (src/testing/diff_ops.rs) cite
# their binding checks as bare strings. The in-crate suite holds those names
# to the live law/descriptor tables and to a #[test]-attribute source scan;
# what no source scan can attest is *collection* — a test in a module the
# tree never wires, or behind a cfg the gate never lights, scans fine and
# never runs. tools/citecheck closes that seam: every citation must resolve,
# by exact name only (final `::` segment for tests, registered name for laws
# and descriptors), against the runner's own inventory from
# `cargo nextest list`, with per-source extraction floors and a self-test
# pinning the red paths. The listing is captured whole to a file and judged
# from the artifact; the verdict is deterministic under any load. The list
# invocation builds test binaries in the root target/, which is why this leg
# rides the workspace stream, after test-all has warmed those artifacts.

# Resolve every roster, bespoke, and adequacy-tripwire citation against the
# collected test inventory.
citecheck:
    ./tools/citecheck --self-test
    mkdir -p target && {{ justfile_directory() }}/tools/memwatch bash -c 'cargo nextest list -p before --all-features --message-format json > target/citecheck-tests.json'
    ./tools/citecheck --tests target/citecheck-tests.json --root crates/before

# The supply-chain leg, two build-free checks over the committed lockfiles.
# cargo-audit sweeps every lockfile in the repository — the root workspace
# and each detached workspace (fuzz, fuzzfit, fuelscape, surfacecheck) —
# against the RustSec advisory database (fetched over the network, so this
# is the one gate leg that needs connectivity): a vulnerability anywhere
# fails the gate; unmaintained/unsound/yanked advisories print as warnings
# for triage without failing. cargo-deny holds the root workspace's
# resolved graph (all members, all features, all targets) to one version
# per crate; deny.toml is the roster of record, every tolerated duplicate
# carrying the holdout that keeps it alive. The duplicate policy covers
# the root workspace only: a duplicate in a detached dev-tooling workspace
# costs one extra tool compile, never a shipped byte. Both invocations
# name `--workspace` explicitly — the root manifest is a package AND a
# workspace, and cargo-deny's default member selection would otherwise
# silently check the root package alone (measured: 84 crates vs 459).
# Needs cargo-audit and cargo-deny: `cargo install cargo-audit cargo-deny`.

# Audit advisories on every lockfile and hold the workspace to single crate versions.
supply-chain:
    cargo audit
    cargo audit --file crates/before/fuzz/Cargo.lock
    cargo audit --file crates/before/fuzzfit/Cargo.lock
    cargo audit --file crates/before-fuelscape/Cargo.lock
    cargo audit --file crates/before/surfacecheck/Cargo.lock
    cargo deny --workspace check bans

# The fuzz targets live in a detached workspace (crates/before/fuzz), so no
# workspace-wide build reaches them and nothing but this leg holds them to
# the API they assert against. It is a gate leg rather than a sweep leg
# because the drift it catches is caused by ordinary refactors — a rename in
# `before` breaks a fuzz target in the same commit that lands it, and a
# compile is seconds of gate time. Only the build: the libFuzzer smoke is
# poor per-commit spend and runs at `just all` cadence.
# Needs cargo-fuzz: `cargo install cargo-fuzz`.

# Build the libFuzzer targets (nightly). The fmt line is the detached
# workspace's formatting leg: the root `cargo fmt --all` cannot reach it.
[working-directory("crates/before/fuzz")]
fuzz-build:
    cargo fmt --check
    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

# The gate runs in two tiers. First `gate-lints`, sequential and
# build-free, because a formatting slip must not cost four minutes to
# learn about. Then `gate-streams`, which is every leg that builds
# something, run concurrently.
#
# What makes the concurrency safe is that nothing in the parallel tier
# judges a nondeterministic quantity: the board's counters, the wasmtime
# fuel bands, the fuelscape sampler pins, and the protocol suites'
# virtual-time assertions all read the same under any machine load, so a
# neighbor stream cannot flake a verdict. The one leg that does judge wall
# time, the bench judge, is deliberately absent — it lives at
# `just bench-judge` / `just all` cadence, where it can have the machine
# to itself.
#
# The stream grouping is not arbitrary. Two cargo invocations sharing a
# target directory serialize on its build lock, so every leg reaching the
# root `target/` sits in one stream and runs in order there, cheap-first,
# preserving the fail-fast ordering within it. Every other leg already
# writes a directory nothing else touches — the nightly doctest target,
# the private-items doc target, the rustdoc-JSON target, and the detached
# fuzz/fuzzfit/fuelscape/surfacecheck workspaces — and that is exactly what
# lets them overlap. `fuzzfit` and `fuelscape-test` share one stream
# because both build the same wasm guest.
#
# Concurrency multiplies peak memory, not just cores: every build-heavy
# leg runs under tools/memwatch, whose per-process ceiling and global swap
# backstop are scoped so concurrent instances never kill each other's
# compiles. A stream that trips it fails loudly and names the crate.

# Run the pre-commit gate; it must come up fully clean before every commit.
gate: gate-lints gate-streams

# The build-free tier, sequential: a lint failure should cost seconds.
gate-lints: fmt-check doclint testdoc readme-check

# Each stream's output is captured rather than interleaved, and a failing
# stream's log is replayed in full at the end, so a parallel failure reads
# like a sequential one. Completion lines arrive live, in finish order.
#
# The verdict has a liveness floor: every stream records its own completion
# (an ok or failed marker), and the verdict requires completions plus
# failures to account for every stream launched. A stream killed from
# outside — the OOM killer, a stray signal — records neither marker, and
# that silence must read as a gate failure, never as a pass.

# Run every building gate leg, grouped into streams that cannot collide.
gate-streams:
    #!/usr/bin/env bash
    set -uo pipefail
    cd "{{ justfile_directory() }}"
    logs=target/gate-logs
    rm -rf "$logs"
    mkdir -p "$logs"
    # The parent's stdout, so a background stream can report completion to
    # the terminal while its own output goes to its log.
    exec 3>&1

    run_stream() {
        name=$1
        niceness=$2
        shift 2
        began=$SECONDS
        rc=0
        for leg in "$@"; do
            echo "===== just $leg ====="
            if ! nice -n "$niceness" just "$leg"; then rc=1; break; fi
        done
        # The marker is the record and the echo is narration, so the marker
        # is written first: a stream that reported ok has already banked it.
        if [ "$rc" -eq 0 ]; then
            : > "$logs/$name.ok"
            echo "gate: ok      $name ($((SECONDS - began))s)" >&3
        else
            : > "$logs/$name.failed"
            echo "gate: FAILED  $name ($((SECONDS - began))s)" >&3
        fi
    }

    streams=()
    start_stream() {
        name=$1
        niceness=$2
        shift 2
        streams+=("$name")
        run_stream "$name" "$niceness" "$@" > "$logs/$name.log" 2>&1 &
        echo "gate: start   $name ($*)"
    }

    # The workspace stream carries the test suite and is the critical
    # path; every other stream together is less work than it is, and all
    # of them finish first even when yielding. So they run niced: the
    # tests keep first call on the cores, and the shorter streams fill
    # what the tests leave idle instead of competing for it.
    began=$SECONDS
    start_stream workspace     0 clippy clippy-default docs test-all citecheck
    start_stream doctest      10 doctest
    start_stream board        10 amp-board-acceptance worst-cases-pin
    start_stream wasm         10 fuzzfit fuelscape-test
    start_stream fuzz         10 fuzz-build
    start_stream surface      10 surface-totality
    start_stream internal-docs 10 docs-internal
    start_stream audit        10 supply-chain
    wait

    failed=$(cd "$logs" && ls *.failed 2>/dev/null | sed 's/\.failed$//')
    # The liveness floor: a stream that recorded no completion at all died
    # without a verdict, which fails the gate exactly as a failure marker
    # would — its partial log is the only witness, so replay it too.
    missing=""
    for name in "${streams[@]}"; do
        [ -e "$logs/$name.ok" ] || [ -e "$logs/$name.failed" ] || missing="$missing $name"
    done
    if [ -n "$failed" ] || [ -n "$missing" ]; then
        for name in $failed; do
            echo
            echo "───── $name ─────"
            cat "$logs/$name.log"
        done
        for name in $missing; do
            echo
            echo "───── $name (died without a verdict) ─────"
            cat "$logs/$name.log"
        done
        summary=""
        [ -n "$failed" ] && summary=" $(echo $failed | tr '\n' ' ')"
        [ -n "$missing" ] && summary="$summary died without a verdict:$missing"
        echo
        echo "gate: FAILED after $((SECONDS - began))s:$summary"
        exit 1
    fi
    echo "gate: clean in $((SECONDS - began))s; stream logs in $logs"

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
    cargo check -p before --no-default-features --features doc-images
    cargo check -p rumors --no-default-features
    cargo check -p rumors --features protocol-v1
    cargo check -p rumors --features meter
    cargo check -p rumors --no-default-features --features conformance

# The viz engine must keep compiling for its real target, not just the host.
wasm-check:
    cargo check -p before-viz --target wasm32-unknown-unknown

# This is exactly what the Pages deploy runs. Needs npm (network on first run).

# Full visualizer build: wasm-pack, strict TypeScript typecheck, esbuild bundle.
viz:
    ./crates/before-viz/build.sh

# The only build that exercises the bench/release profile; benches otherwise rot silently.

# Compile (don't run) the criterion benches.
bench-build:
    {{ justfile_directory() }}/tools/memwatch cargo bench --workspace --no-run

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
    # libFuzzer requires its write-corpus directory to exist, and the
    # discovery corpus is deliberately untracked (fuzz/.gitignore), so a
    # fresh checkout must create the directories before the first run.
    mkdir -p corpus/fuzz_decode corpus/fuzz_decode_differential corpus/fuzz_decode_ops corpus/fuzz_laws corpus/fuzz_parse
    cargo +{{ nightly_toolchain }} fuzz run --target {{ host_triple }} fuzz_decode corpus/fuzz_decode seeds/fuzz_decode -- -max_total_time={{ secs }}
    cargo +{{ nightly_toolchain }} fuzz run --target {{ host_triple }} fuzz_decode_differential corpus/fuzz_decode_differential seeds/fuzz_decode_differential -- -max_total_time={{ secs }}
    cargo +{{ nightly_toolchain }} fuzz run --target {{ host_triple }} fuzz_decode_ops corpus/fuzz_decode_ops seeds/fuzz_decode_ops -- -max_total_time={{ secs }}
    cargo +{{ nightly_toolchain }} fuzz run --target {{ host_triple }} fuzz_laws corpus/fuzz_laws seeds/fuzz_laws -- -max_total_time={{ secs }}
    cargo +{{ nightly_toolchain }} fuzz run --target {{ host_triple }} fuzz_parse corpus/fuzz_parse seeds/fuzz_parse -- -max_total_time={{ secs }}

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
    cargo build -p fuzzfit-guest --release --target wasm32-unknown-unknown --target-dir {{ fuzzfit_target }}
    {{ justfile_directory() }}/tools/memwatch cargo build -p fuzzfit-harness --tests --release

# Run the fuzz-fit suites: generator sanity, meter liveness, the judgment
# and shape-leg tripwires, the quadratic-burner adequacy check, the
# toolchain-pin and staleness cross-checks, and the enforcement sentry
# (48 fuzzed programs against the pinned bands, point and shape legs,
# plus the whole 256-program deterministic prefix judged step by step:
# the random draws probe novelty, the prefix leg is total). A failure
# shrinks to a minimal out-of-band shape and writes a proptest seed
# file — commit any seed that appears.

# Run the fuzz-fit asymptotics suites against the pinned fuel bands. The
# fmt/clippy lines are the detached workspace's own lint leg (the root
# `cargo fmt --all`/clippy cannot reach a detached workspace, so without
# them its source rots invisibly through green gates — the fuelscape and
# surfacecheck recipes carry the same discipline).
[working-directory("crates/before/fuzzfit")]
fuzzfit: fuzzfit-build
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} {{ justfile_directory() }}/tools/memwatch cargo nextest run --cargo-profile release

# Re-fit the pinned bands from the committed deterministic corpus (4096
# programs; byte-reproducible, so any diff is a real change). Rewrites
# harness/src/bands.rs atomically: review the diff like a snapshot and
# commit with the movement annotated in the commit message.

# Re-fit and rewrite the fuzz-fit harness's pinned fuel bands.
[working-directory("crates/before/fuzzfit")]
fuzzfit-calibrate: fuzzfit-build
    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo run --release -p fuzzfit-harness --bin calibrate

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
    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} {{ justfile_directory() }}/tools/memwatch cargo nextest run

# Renders one log-log heatmap per public operation into target/fuelscape
# (SVG per op plus a gallery index.html): p(fuel | size) from uniform
# draws over each exact-byte-size canonical input space, the committed
# adversarial families overlaid as marked points, wasmtime instruction
# fuel as the work currency. Deterministic per (seed, plan): re-running
# the same plan on the same guest reproduces every reading. Defaults:
# 300 samples/column on average (each row's budget is spread-weighted
# toward its larger columns) to 256 bytes; override e.g.
# `just fuelscape --samples 500 --max-bytes 512`.

# Render the full population atlas into target/fuelscape (audit view; not enforcement).
[working-directory("crates/before-fuelscape")]
fuelscape *args: fuzzfit-build
    FUELSCAPE_TIP=$(git rev-parse HEAD) FUZZFIT_GUEST_WASM={{ justfile_directory() }}/target/fuzzfit/wasm32-unknown-unknown/release/fuzzfit_guest.wasm cargo run --release --bin fuelscape -- --out {{ justfile_directory() }}/target/fuelscape {{ args }}

# The rustdoc fuelscape islands' committed inputs are derived artifacts:
# the widget datasets (crates/before/fuelscape) derive by pure compaction
# from the committed gzipped dump (crates/before-fuelscape/dump), and the
# doc header derives from the widget stylesheet and script by
# concatenation. Every derivation carries a freshness pin — before's
# build.rs refuses a stale header outright, and fuelscape-verify
# re-derives the datasets and byte-compares (ci tier: the strict dump
# read prices about a minute). Re-measuring is a deliberate re-pin:
# `just fuelscape --dump --samples <n> --max-bytes <n>`, gzip the dump
# into crates/before-fuelscape/dump (`gzip -9 -n`, byte-stable), then
# `just fuelscape-compact`, and commit dump, datasets, and any roster
# claim changes together.

# Re-derive the fuelscape widget datasets from the committed dump.
[working-directory("crates/before-fuelscape")]
fuelscape-compact:
    cargo run --release --bin fuelscape -- --compact-from dump --out {{ justfile_directory() }}/crates/before/fuelscape

# Verify the committed widget datasets equal a fresh compaction of the committed dump.
[working-directory("crates/before-fuelscape")]
fuelscape-verify:
    rm -rf {{ justfile_directory() }}/target/fuelscape-verify
    cargo run --release --bin fuelscape -- --compact-from dump --out {{ justfile_directory() }}/target/fuelscape-verify
    diff -r {{ justfile_directory() }}/target/fuelscape-verify {{ justfile_directory() }}/crates/before/fuelscape

# Regenerate the rustdoc widget header from the committed stylesheet and script.
fuelscape-header:
    { printf '<style>'; cat crates/before/docs/fuelscape.css; printf '</style>\n<script>'; cat crates/before/docs/fuelscape.js; printf '</script>\n'; } > crates/before/docs/fuelscape-header.html

# The widget bundle and the claim strings meet only in the reader's
# browser, so nothing compiled checks them; this leg loads the bundle
# under node and parses every committed claim with the widget's own
# exported grammar (ci tier: node is already a ci prerequisite, and the
# gate stays node-free).

# Check the widget bundle's syntax and every committed claim's grammar.
fuelscape-claims:
    node --check crates/before/docs/fuelscape.js
    ./tools/fuelscape-claims

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

# One allocation-strategy A/B run (benches/presize.rs):
# compiles the named arm into the library — "shipped" compiles no cfg (the
# shipped library, byte-identical); any other arm goes in through
# `--cfg before_alloc_ab="<arm>"`, so expect a full rebuild per arm switch —
# and saves the criterion baseline `<target>-<arm>`, which is where the
# side's label lives (nothing in-process distinguishes the sides; each
# binary also prints its compiled arm as provenance). Record protocol: one
# "shipped" run per target at full sampling, then one run per site arm
# filtered to that site's cells; compare baselines pairwise per site. The
# recipe validates the arm name (a mistyped arm sets a cfg nothing queries,
# which would silently benchmark the shipped code under a mislabeled
# baseline); the same roster is registered as check-cfg values in
# crates/before/Cargo.toml, whose `deny` keeps roster and seams in sync.
# Reduced-sampling smoke: append `--sample-size 10 --measurement-time 1`
# (never quoted).
bench-alloc-ab target arm="shipped" *filter:
    @case "{{ arm }}" in (shipped|projection_growth|projection_shrink|display_growth) ;; (*) echo 'bench-alloc-ab: unknown arm "{{ arm }}"' >&2; exit 2;; esac
    RUSTFLAGS='{{ if arm == "shipped" { "" } else { '--cfg before_alloc_ab="' + arm + '"' } }}' cargo bench -p before --bench {{ target }} -- --save-baseline {{ target }}-{{ arm }} {{ filter }}

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
# 1.7 for the conversion-dominated text-IO cells (the wide-display pair,
# the hugeleaf display pair, and the hugeleaf parse trio), the class
# declared per cell by the bench sidecar, never by the roster — red/green
# table. Every run judges through
# the committed roster (tools/benchjudge-expected.json: expected reds by
# cell name — exactly the permanent schoolbook tripwire, required RED at
# its text ceiling; tests/bench_judge_roster.rs pins the exact
# membership; the sampling pin covers both modes — the expectations are
# exponent classes, which hold under either regime), so it passes on the
# honest tree and fails on any unexpected red OR unexpected green.

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
# order. Every judged quantity is a deterministic counter read from a
# per-process allocator or a per-process global, so a reading is a
# function of its cell alone: neither machine load nor the shard layout
# that measured it can move one. AMP_BOARD_SHARDS overrides the count.
#
# A bare run (any single scale) is a debugging view whose verdicts never
# bind; the verdict of record is the acceptance invocation below, which
# measures each cell's whole ladder in one judgment. Optional scale
# multiplies the input sizes, e.g. `just amp-board 4`.

# Render the amplification board at one scale: a debugging view of the red-green matrix.
amp-board *args:
    cargo run --release -p before --example amp_board --features limb-meter,scan-meter -- {{ args }}

# The board's one verdict of record: one invocation measures each cell's
# whole ladder — two sizes at each of the two sampling scales
# (board::DEFAULT_SCALE and board::LADDER_TOP_SCALE, the segment-onset
# witness) — judges every constant and floor per size, fits each
# exponent as one trend across the ladder, and exits nonzero on any red
# cell. A red is an untriaged contradiction, resolved only by a cure or
# an owner-declared model at the cell, so the gate's board stream fails
# on any red. Every judged quantity is a deterministic counter, so a
# second run reads the same board and acceptance needs no repeated hand
# runs.

# Run the board's acceptance judgment: the whole measurement ladder, one verdict.
amp-board-acceptance:
    cargo run --release -p before --example amp_board --features limb-meter,scan-meter -- acceptance

# The surface-totality leg: the operation roster in
# crates/before/src/surface.rs (METHOD_SURFACE, the machine-readable
# enumeration the surface-coverage suite enforces) is held total against
# nightly rustdoc JSON — the compiler's own account of the public
# surface — so a public fn or method added anywhere (a new file, a new
# module, a feature-gated tree) fails the gate until it gains a roster
# row or a named, reasoned exception in crates/before/surfacecheck. Trait
# impls (operators, codecs, derives) and non-function items (associated
# consts and types, statics, macros) are held to the same standard by
# the pinned censuses in crates/before/surfacecheck/src/census.rs,
# reconciled both ways, so a new impl or item reads red until pinned. The
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
# the claim that this is the true worst case is carried by the rustdoc
# complexity sections and the asymptotics liveness pins, not by this
# table. Runs at release, the board's profile of record, one table per
# sampling scale of the ladder.

# Render the worst-case map: the argmax family per operation x currency, one table per sampling scale.
worst-cases:
    cargo run --release -p before --example amp_board --features limb-meter,scan-meter -- worst-cases

# The map's rankings are pinned: a committed expectation table (the
# WORST_RANKINGS const beside the fold) is entry-compared against the live
# fold at both of the ladder's sampling scales, so a ranking flip is caught in the gate,
# never discovered by a reader. A flip is news: either a family
# legitimately overtook (re-pin deliberately with a movement annotation) or
# a code change made some shape relatively worse (investigate first).
# Exits nonzero on any drift, naming the operation, currency, scale, and
# both worsts. Runs at release, the board's profile of record: rankings
# derive from readings, and dev readings are never pinned.

# Entry-compare the live worst-case fold against the committed ranking pin.
worst-cases-pin:
    cargo run --release -p before --example amp_board --features limb-meter,scan-meter -- worst-cases-check

# ── the no-rot sweep ─────────────────────────────────────────────────────────
# `ci` is the build-everything tier: formatting and lints, the feature matrix,
# wasm, docs, the full test+doctest run, bench builds, the fuzz-target *build*,
# and the viz bundle, ordered cheap-first so failures surface early. GitHub CI
# runs exactly this. Neither `ci` nor `all` runs the gate's instrument legs —
# the fuel bands, the fuelscape pins, the board's acceptance verdicts and
# ranking pin, and surface
# totality run in `just gate` (its recipe line is the
# roster of record), pre-commit on a developer machine; GitHub CI's
# `instruments` job re-runs the counter-based subset (board verdicts, the
# ranking pin, surface totality, and the supply-chain leg) beside
# the `ci` sweep, leaving the wall-time judge and the wasm fuel tier local.
# CI's `coverage` job carries the two instrumented-coverage legs (the
# coverage section below): too slow for the gate, judged against the
# curated kernel pin.
#
# `all` is `ci` plus what CI cannot run: a short libFuzzer smoke (poor
# per-commit spend), the formal tier (the runner has no Lean toolchain) —
# the kernel-checked proofs, the eventdag oracle/schedule gate, and the
# muxprobe matrix gate — and the bench judge's two legs: the roster-mode
# judgment (minutes of criterion runs at two scales; quick mode, so its
# exponents are judged but never quoted) and the seconds-scale live
# tripwire, so the judge's red path rides every sweep.

# Build everything (no fuzz run): the no-rot sweep as CI runs it.
ci: fmt-check doclint testdoc readme-check fuelscape-claims clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz

# Everything: the no-rot sweep, plus the fuzz smoke, the formal tier, and the bench judge.
all: ci (fuzz fuzz_smoke_secs) lean eventdag muxprobe bench-judge bench-judge-tripwire

# ── the coverage legs (CI cadence; the gate never runs them) ─────────────────
# GOAL: no skyline-kernel arm goes silently unexercised — every uncovered
# kernel line and every untaken branch direction is either curated (a
# panic-arm or an unreachable arm, its argument stated at the entry) or a
# named remediation item, and any NEW hole fails by name. MECHANISM:
# cargo-llvm-cov produces an lcov report; tools/covcheck holds it to the
# curated pinned expectation in tools/covcheck-expected.json, tamper-evident
# in both directions — a new uncovered kernel line fails, and a stale entry
# (covered, gone, or no longer instrumented) fails until the pin tightens.
# Deliberately NOT a global coverage threshold: the worst artifact passing a
# threshold is a suite that pads covered lines elsewhere; the pin names lines.
#
# CI legs, never gate legs: each run is a full instrumented rebuild plus the
# whole suite under instrumentation — minutes, not gate seconds. The line leg
# runs on stable; branch instrumentation needs the pinned nightly (the same
# toolchain-pin argument as the other nightly legs, and each leg judges only
# its own toolchain's records — the two map a few regions to different
# lines). One residual to know when a red arrives: proptest populations draw
# fresh cases each run, so an arm a random case occasionally grazes can flip
# a pinned line to covered. That red is information, not noise — the arm is
# reachable, so promote its remediation entry to a directed test family and
# remove it. Needs cargo-llvm-cov: `cargo install cargo-llvm-cov`.

covcheck_expected := justfile_directory() + "/tools/covcheck-expected.json"

# Run the instrumented suite (stable) and hold kernel line coverage to the pin.
coverage-kernel:
    ./tools/covcheck --self-test
    @mkdir -p target/llvm-cov
    {{ justfile_directory() }}/tools/memwatch cargo llvm-cov nextest --workspace --all-features --lcov --output-path target/llvm-cov/workspace.lcov
    ./tools/covcheck --lcov target/llvm-cov/workspace.lcov --expected {{ covcheck_expected }} --root {{ justfile_directory() }}

# Run the instrumented suite (pinned nightly, --branch) and hold kernel branch coverage to the pin.
coverage-kernel-branch:
    ./tools/covcheck --self-test
    @mkdir -p target/llvm-cov
    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} llvm-cov nextest --branch --workspace --all-features --lcov --output-path target/llvm-cov/workspace-branch.lcov
    ./tools/covcheck --branch --lcov target/llvm-cov/workspace-branch.lcov --expected {{ covcheck_expected }} --root {{ justfile_directory() }}
