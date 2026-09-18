# rumors workspace: the source of truth for verification. Every artifact in
# the workspace has a recipe here, tiered by feedback speed, and `just --list`
# is the tour. The one exception class is the hand-run instrument: an
# `#[ignore]`-gated test no recipe runs, whose own doc states the cost that
# keeps it out and the command that runs it.
#
#   inner loop   just check / just test <filter>     seconds to a minute
#   commit gate  just gate                           fully clean before every commit
#   no-rot sweep just ci / just all                  everything, so nothing rots
#
# The gate runs every check a commit must pass: build-free lints first,
# then every building leg concurrently (see the comment above `gate` for
# the stream grouping and why parallelism cannot move a verdict).
# `ci` is the recipe GitHub CI's `ci` job runs: the gate's lints and tests
# plus the artifacts the gate doesn't reach (the feature matrix, wasm, bench
# builds, the viz bundle). `all` adds the coverage legs (CI's `coverage`
# job) and what CI cannot run (the fuzz smoke and formal tier). Neither sweep
# repeats the gate's instrument legs -- the fuel
# bands, the board verdicts and pins, and surface totality run in
# `just gate`, and GitHub CI's `instruments` job re-runs the counter-based
# subset (the workflow file says which legs stay local and why). The
# comment above each recipe states what it verifies and why.

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

# Default cases per property in CI's release-profile Rumors run. Four thousand
# takes the slowest current property about five to six minutes on Helios, within
# the ten-minute limit in .config/nextest.toml. Re-measure before raising it.
proptest_ci_cases := "4000"

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

# The 32-bit boundary-pin guest's wasm: produced by `wasm32-pins-build`,
# read by the `wasm32-pins` harness run. Named and passed explicitly for
# the same reason as the fuzz-fit guest above: an ambient CARGO_TARGET_DIR
# must not separate the producer from the consumer.

wasm32pins_target := justfile_directory() + "/target/wasm32-pins"
wasm32pins_guest_wasm := wasm32pins_target + "/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm"

# List recipes.
default:
    @just --list --unsorted

# ── inner loop ───────────────────────────────────────────────────────────────

# Type-check every host target: libs, tests, benches, examples. A target
# with `required-features` (the before envelope suite) is skipped here and
# type-checked by `just test` and the gate's all-features legs instead.
check:
    cargo check --workspace --all-targets

# Run the test suites; pass a filter to narrow (`just test mirror`). The
# envelope suite (crates/before/tests/meter.rs) builds only with its touch
# and scan meters (`required-features` on its test target), so the inner
# loop lights them for that package. One `--workspace` build unifies
# features, so this compiles the before lib with both meters for every
# dependent and runs every before suite gated on them (the envelope
# suite, the fold and coincident-span suites, the touch-meter unit tests);
# the other packages' own features stay default.
test *args:
    cargo nextest run --workspace --features before/touch-meter,before/scan-meter {{ args }}

# Every feature is lit here and nowhere else in the gate: the meter suites
# and the conformance module build only under `--all-features`.

# Run the test suites under every feature (the gate's test run).
test-all *args:
    cargo nextest run --workspace --all-features --no-fail-fast {{ args }}

# Run Rumors at a larger property-test count under the faster release profile.
# This stays package-scoped: before's operation meters intentionally pin the
# dev profile and have their own exhaustive verification. A few unusually
# expensive stress properties retain their explicit, smaller case budgets.
test-release *args:
    PROPTEST_CASES={{ proptest_ci_cases }} cargo nextest run -p rumors --all-features --locked --cargo-profile release --profile high-count --no-fail-fast {{ args }}

# Stable rustdoc compiles one executable per example; `before` has nearly 100,
# and their macOS link work dominates the gate. Nightly's merged mode compiles
# one harness per crate instead. Keep its target separate so switching compilers
# cannot invalidate the stable gate artifacts (or vice versa).

# Run the doctests (nightly), which nextest does not run.
doctest:
    RUSTDOCFLAGS="-Z unstable-options --merge-doctests yes" cargo +{{ nightly_toolchain }} test --workspace --doc --all-features --target-dir target/doctest-nightly

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
#
# The bare-lib rumors line lints the shipped configuration on its own: an
# invocation that also builds test targets compiles the lib with
# cfg(test)-gated modules alive, so an item dead only in the default-feature
# lib — the artifact users actually build — never surfaces there.

# Lint the default-feature library and test builds, warnings denied.
clippy-default:
    cargo clippy -p suanpan --lib --tests -- -D warnings
    cargo clippy -p before --lib --tests -- -D warnings
    cargo clippy -p rumors --lib --tests -- -D warnings
    cargo clippy -p rumors --lib -- -D warnings

# Format the whole workspace.
fmt:
    cargo fmt --all

# Verify formatting without rewriting (the gate variant of `fmt`).
fmt-check:
    cargo fmt --all --check

# ── commit gate: everything a commit must pass, fully clean ──────────────────

# tools/doclint enforces the two doc-comment layout rules rustdoc renders by
# but never reports on. First, a doc comment's opening paragraph becomes the
# item's summary — the one-liner in module index tables and search — so it
# fails the gate when that paragraph grows past one; the fix is to move the
# rest below a blank `///` line. Second, an `#[doc = include_str!(…)]` needs a
# blank doc line between it and the doc prose on each side, or Markdown never
# gives the included file a block of its own: a figure spliced into an open
# paragraph collects that paragraph's `</p>` mid-SVG and vanishes from the
# rendered page, and prose left inside an unclosed HTML block renders as its
# literal source. The tool's own header carries both derivations.
#
# It covers every Rust source in the workspace (libraries, tests, benches,
# examples, the demo crate), and it needs no build, so it runs first for fast
# failure. `--self-test` leads, so a bug in the checker surfaces as a tool
# failure rather than as a silent all-clear.

# Flag doc-comment summaries and included-figure layouts that render wrong.
doclint:
    ./tools/doclint --self-test
    ./tools/doclint benches crates examples src tests

# tools/testdoc checks the Rust files under the five roots doclint walks,
# minus the directory names testdoc's ignore set prunes; it never walks `.`,
# whose untracked trees (other agents' worktrees under `.claude/`) would
# otherwise enter the verdict.

# Require every Rust test to document the behavior and invariant it protects.
testdoc:
    ./tools/testdoc --self-test
    ./tools/testdoc benches crates examples src tests

# No other gate leg polices what the CI workflows themselves execute:
# tools/workflowlint holds every workflow step to committed or
# immutably-pinned code (its docstring carries the full argument).
# Build-free, so it rides the lint tier.

# Hold every workflow step to committed or immutably-pinned code.
workflowlint:
    ./tools/workflowlint --self-test
    ./tools/workflowlint .github

# tools/manifestlint holds every member manifest to the workspace dependency
# table: each dependency entry inherits with `workspace = true`, adding only
# feature selections and `optional` flags, so the table stays the one version
# of record and no member-local restatement can drift when it moves. Cargo
# has no native check (`cargo metadata` resolves inheritance away before
# reporting), so the tool reads the member manifests cargo names.
# Build-free, so it rides the lint tier.

# Hold every member manifest's dependencies to the workspace table.
manifestlint:
    ./tools/manifestlint --self-test
    ./tools/manifestlint

# tools/digestshare reads the committed V2 wire captures and totals digest
# vs non-digest bytes. As a gate leg it checks the renderer-vocabulary
# contract, not a threshold: the tool exits nonzero when the corpus's
# byte-count headers or listing entries stop matching its patterns (the
# renderer's vocabulary moved out from under the meter), never on the
# measured ratio. Build-free, so it rides the lint tier.

# Check the wire-capture renderer vocabulary via the digest-share meter.
digestshare:
    ./tools/digestshare --self-test
    ./tools/digestshare

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
# Needs cargo-rdme at the version tools/readme pins (the READMEs are
# byte-pinned derived artifacts, so the deriving tool is pinned too; the
# tool refuses a mismatch and names the install command).

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
# The fuelscape widget (the interactive measured-growth explorers in
# before's # Complexity sections) needs its stylesheet and script on every
# page that holds a chart, and a page that lacks them shows each chart as
# an empty expander and fails nothing else. The assets travel inside the
# doc comments of the items owning those pages (before's build.rs writes
# the fragment), so every rustdoc render carries them, a bare `cargo doc`
# and docs.rs included; tools/fuelscape-assets then holds the rendered
# pages to that placement: the assets exactly once on every page with a
# chart, and on no other.

# Build the rustdoc with warnings denied; hold the widget assets to their pages.
docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
    ./tools/fuelscape-assets --self-test
    ./tools/fuelscape-assets target/doc

# The public build above never renders private items, so a stale intra-doc
# link inside a private module sails through it. This pass documents private
# items too. It cannot replace `docs`: with private items rendered, the
# `private_intra_doc_links` lint (public docs linking to a private item) no
# longer fires, so each pass catches a class the other cannot. A separate
# target dir keeps the two from invalidating each other's fingerprints, which
# would otherwise re-doc the whole workspace twice on every gate.

# Build the rustdoc including private items, warnings denied; hold the widget assets to their pages.
docs-internal:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --document-private-items --target-dir target/doc-internal
    ./tools/fuelscape-assets --self-test
    ./tools/fuelscape-assets target/doc-internal/doc

# docs.rs builds under nightly with `--cfg docsrs`, the one configuration
# where rumors' `doc_cfg` gate is live; `docs` and `docs-internal` run
# stable rustdoc without it, so only this leg compiles the path docs.rs
# takes. The first line is that build for rumors with warnings denied. The
# `cargo docs-rs` lines then imitate docs.rs's own invocation for each
# crate from its `[package.metadata.docs.rs]` (features, cfg, scraped
# examples, the docs.rs extern map), the faithful check that the metadata
# builds; the pass over its output holds before's widget assets to their
# pages under that configuration too. All under the pinned nightly, no
# deps, in one target dir of its own. What this cannot check is the
# package: docs.rs builds the published tarball, and `cargo package` on
# before refuses until its `suanpan` dependency carries a version.

# Build the rustdoc as docs.rs does (pinned nightly, `--cfg docsrs`, each crate's docs.rs metadata).
docs-docsrs:
    RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +{{ nightly_toolchain }} doc -p rumors --all-features --no-deps --target-dir target/doc-docsrs
    cargo +{{ nightly_toolchain }} docs-rs -p rumors --target-dir target/doc-docsrs
    cargo +{{ nightly_toolchain }} docs-rs -p before --target-dir target/doc-docsrs
    ./tools/fuelscape-assets target/doc-docsrs/{{ host_triple }}/doc

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

# tests/future_size.rs pins the public futures' sizes, and a `cfg` that
# compiles it empty would read as a pass. This leg reruns that binary from
# test-all's build set (same features, so no rebuild) with
# `--no-tests=fail`, so an empty binary fails the gate. The filter names
# the binary by its package-scoped id, so a same-named binary in another
# member cannot keep the leg green while rumors' collects nothing.

# Fail unless the future-size pins were collected and pass (liveness for tests/future_size.rs).
future-size:
    cargo nextest run --workspace --all-features -E 'binary_id(rumors::future_size)' --no-tests=fail

# Resolve every roster, bespoke, and tripwire citation against the collected test inventory.
citecheck:
    ./tools/citecheck --self-test
    mkdir -p target && cargo nextest list -p before --all-features --message-format json > target/citecheck-tests.json
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
# poor per-commit spend and runs at `just all` cadence. The fmt line is
# the detached workspace's formatting leg: the root `cargo fmt --all`
# cannot reach it.
# Needs cargo-fuzz: `cargo install cargo-fuzz`.

# Build the libFuzzer targets (nightly), with the detached workspace's fmt check.
[working-directory("crates/before/fuzz")]
fuzz-build:
    cargo fmt --check
    cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

# The gate runs in two tiers. First `gate-lints`, sequential and
# build-free, because a formatting slip must not cost four minutes to
# learn about. Then `gate-streams`, which is every leg that builds
# something, run concurrently.
#
# What makes the concurrency safe is that every verdict is deterministic:
# the board's counters, Wasmtime fuel bands, fuelscape sampler pins, and the
# protocol suites' virtual-time assertions are unaffected by machine load.
#
# The stream grouping is not arbitrary. Two cargo invocations sharing a
# target directory serialize on its build lock, so every leg reaching the
# root `target/` sits in one stream and runs in order there, cheap-first,
# preserving the fail-fast ordering within it. Every other leg already
# writes a directory nothing else touches — the nightly doctest target,
# the private-items doc target, the docs.rs doc target, the rustdoc-JSON
# target, and the detached fuzz/fuzzfit/fuelscape/surfacecheck workspaces
# -- and that is exactly what lets them overlap. `fuzzfit` and
# `fuelscape-test` share one stream because both build the same wasm guest.
#
# Concurrency multiplies peak memory, not just cores. Nothing here caps
# memory; how many gates share a machine is the operator's call.

# Run the pre-commit gate; it must come up fully clean before every commit.
gate: gate-lints gate-streams

# The build-free tier, sequential: a lint failure should cost seconds.
gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare readme-check

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
    start_stream workspace     0 clippy clippy-default docs test-all future-size citecheck
    start_stream doctest      10 doctest
    start_stream board        10 amp-board-acceptance worst-cases-pin
    start_stream wasm         10 fuzzfit fuelscape-test wasm32-pins
    start_stream fuzz         10 fuzz-build
    start_stream surface      10 surface-totality
    start_stream internal-docs 10 docs-internal
    start_stream docsrs       10 docs-docsrs
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
    cargo check -p before --no-default-features --features touch-meter
    cargo check -p before --no-default-features --features scan-meter
    cargo check -p before --no-default-features --features serde,borsh
    cargo check -p rumors --no-default-features
    cargo check -p rumors --no-default-features --features fs
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
    cargo bench --workspace --no-run

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
    mkdir -p corpus/fuzz_decode corpus/fuzz_decode_differential corpus/fuzz_decode_ops corpus/fuzz_laws
    cargo +{{ nightly_toolchain }} fuzz run --target {{ host_triple }} fuzz_decode corpus/fuzz_decode seeds/fuzz_decode -- -max_total_time={{ secs }}
    cargo +{{ nightly_toolchain }} fuzz run --target {{ host_triple }} fuzz_decode_differential corpus/fuzz_decode_differential seeds/fuzz_decode_differential -- -max_total_time={{ secs }}
    cargo +{{ nightly_toolchain }} fuzz run --target {{ host_triple }} fuzz_decode_ops corpus/fuzz_decode_ops seeds/fuzz_decode_ops -- -max_total_time={{ secs }}
    cargo +{{ nightly_toolchain }} fuzz run --target {{ host_triple }} fuzz_laws corpus/fuzz_laws seeds/fuzz_laws -- -max_total_time={{ secs }}

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
    cargo build -p fuzzfit-harness --tests --release

# Run the fuzz-fit suites: generator sanity, meter liveness, the judgment
# and shape-leg tripwires, the quadratic-burner adequacy check, the
# toolchain-pin and staleness cross-checks, and the enforcement sentry
# (48 fuzzed programs against the pinned bands, point and shape legs,
# plus the whole 256-program deterministic prefix judged step by step:
# the random draws probe novelty, the prefix leg is total). A failure
# shrinks to a minimal out-of-band shape and writes a proptest seed
# file -- commit any seed that appears. The fmt/clippy lines are the
# detached workspace's own lint leg (the root `cargo fmt --all`/clippy
# cannot reach a detached workspace, so without them its source rots
# invisibly through green gates -- the fuelscape and surfacecheck recipes
# carry the same discipline).

# Run the fuzz-fit asymptotics suites against the pinned fuel bands.
[working-directory("crates/before/fuzzfit")]
fuzzfit: fuzzfit-build
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo nextest run --cargo-profile release

# Re-fit the pinned bands from the committed deterministic corpus (4096
# programs; byte-reproducible, so any diff is a real change). Rewrites
# harness/src/bands.rs atomically: review the diff like a snapshot and
# commit with the movement annotated in the commit message.

# Re-fit and rewrite the fuzz-fit harness's pinned fuel bands.
[working-directory("crates/before/fuzzfit")]
fuzzfit-calibrate: fuzzfit-build
    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo run --release -p fuzzfit-harness --bin calibrate

# The detached wasm32 workspace executes `before` at the 32-bit boundaries
# where `usize` conversions or backend capacity can change behavior. Each
# boundary has adjacent cases so a failure identifies the exact transition.
# The guest uses release overflow semantics, so explicit checks must prevent a
# 32-bit wrap from changing the observed result.

# Build the 32-bit boundary-pin wasm guest and its harness (both halves).
[working-directory("crates/before/wasm32-pins")]
wasm32-pins-build:
    cargo build -p wasm32-pins-guest --release --target wasm32-unknown-unknown --target-dir {{ wasm32pins_target }}
    cargo build -p wasm32-pins-harness --tests --release

# The deep pins walk hundreds of megabytes inside a 32-bit guest, so the
# run costs minutes of wall time and peaks at a few GiB of host memory
# across nextest's parallel workers. The fmt/clippy lines are the detached
# workspace's own lint leg (the root `cargo fmt --all`/clippy cannot reach
# a detached workspace, so without them its source rots invisibly through
# green gates -- the fuzzfit recipes carry the same discipline).

# Run the 32-bit boundary pins under wasmtime (minutes; a few GiB of host memory).
[working-directory("crates/before/wasm32-pins")]
wasm32-pins: wasm32-pins-build
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    WASM32_PINS_GUEST_WASM={{ wasm32pins_guest_wasm }} cargo nextest run --cargo-profile release

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
    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo nextest run

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

# Regenerate the README's space-consumption figure from the measurement
# artifact (crates/before/results). The committed file is derived — the
# README references it by URL, so it must exist in the tree — and
# before's build.rs holds it fresh; the env var is the explicit opt-in
# that lets the build write into the source tree.

# Regenerate the README's space-consumption figure from the measurement artifact.
doc-figure:
    BEFORE_REGEN_DOC_FIGURE=1 cargo build -p before

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

# Full sampling is required for any quoted number. The filter matches Criterion
# IDs (`group/function`), for example `just bench version merge` or
# `just bench gossip_grid`.

# Run one bench target through a criterion filter, at full sampling.
bench target *filter:
    cargo bench --workspace --bench {{ target }} -- {{ filter }}

# The reduced-sampling inner loop: `bench` at 10 samples x 1 s. Never quoted.
bench-quick target *filter:
    cargo bench --workspace --bench {{ target }} -- --sample-size 10 --measurement-time 1 {{ filter }}

# Compile one alternative projection allocation strategy and save its
# Criterion baseline. "shipped" selects the normal implementation.

# Run one allocation-strategy A/B arm of a bench target, saving its criterion baseline.
bench-alloc-ab target arm="shipped" *filter:
    @case "{{ arm }}" in (shipped|projection_growth|projection_shrink) ;; (*) echo 'bench-alloc-ab: unknown arm "{{ arm }}"' >&2; exit 2;; esac
    RUSTFLAGS='{{ if arm == "shipped" { "" } else { '--cfg before_alloc_ab="' + arm + '"' } }}' cargo bench -p before --bench {{ target }} -- --save-baseline {{ target }}-{{ arm }} {{ filter }}

# Each board cell judges deterministic work counters (touches, scans,
# segments, heap) against a pinned proportionality envelope: green means
# work scaled with the input, red is an amplification finding. The board
# reads no clock, so its output is byte-identical under any machine load.
# Optional scale multiplies the input sizes, e.g. `just amp-board 4`.
#
# The board runs at the release profile, the profile of record: debug
# assertions perform metered work through probe cursors, so a dev board
# measures algorithm plus verification scaffolding while release measures the
# production work alone. A dev run (`cargo run -p before --example amp_board ...`) remains
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

# One command serves every board recipe, keeping the production profile part
# of the instrument rather than a convention each entry point must repeat.
amp_board_command := "cargo run --release -p before --example amp_board --features touch-meter,scan-meter"

# Render the amplification board at one scale: a debugging view of the red-green matrix.
amp-board *args:
    {{ amp_board_command }} -- {{ args }}

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
    {{ amp_board_command }} -- acceptance

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
    cargo nextest run
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
    {{ amp_board_command }} -- worst-cases

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
    {{ amp_board_command }} -- worst-cases-check

# ── the no-rot sweep ─────────────────────────────────────────────────────────
# `ci` is the build-everything tier: formatting and lints, the feature matrix,
# wasm, docs, the full test+doctest run, bench builds, the fuzz-target *build*,
# and the viz bundle, ordered cheap-first so failures surface early. GitHub
# CI's `ci` job runs exactly this. Neither `ci` nor `all` runs the gate's
# instrument legs -- the fuel bands, the fuelscape pins, the board's
# acceptance verdicts and ranking pin, and surface totality run in
# `just gate` (its recipe line is the roster of record), pre-commit on a
# developer machine; GitHub CI's `instruments` job re-runs the counter-based
# subset (board verdicts, the ranking pin, surface totality, and the
# supply-chain leg) beside the `ci` sweep, leaving the wall-time judge and
# the wasm fuel tier local.
# CI's `coverage` job carries the two instrumented-coverage legs (the
# coverage section below): too slow for the gate, judged against the
# curated kernel pin.
#
# `all` is `ci` plus the high-count property run, coverage legs, and work CI
# cannot run: a short libFuzzer smoke and the formal and model-based gates.

# Build everything (no fuzz run): the no-rot sweep as CI runs it.
ci: fmt-check doclint testdoc workflowlint manifestlint digestshare readme-check fuelscape-claims clippy clippy-default features wasm-check docs docs-internal docs-docsrs test-all future-size citecheck doctest bench-build fuzz-build fuelscape-verify viz

# Everything: the no-rot sweep, coverage, fuzz smoke, and formal/model gates.
all: ci test-release coverage-kernel coverage-kernel-branch (fuzz fuzz_smoke_secs) lean eventdag muxprobe

# ── the coverage legs (`all` and CI cadence; the gate never runs them) ───────
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
# Sweep legs (`all` and CI's `coverage` job), never gate legs: each run is a
# full instrumented rebuild plus the whole suite under instrumentation --
# minutes, not gate seconds. The line leg runs on stable; branch
# instrumentation needs the pinned nightly (the same
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
    cargo llvm-cov nextest --workspace --all-features --lcov --output-path target/llvm-cov/workspace.lcov
    ./tools/covcheck --lcov target/llvm-cov/workspace.lcov --expected {{ covcheck_expected }} --root {{ justfile_directory() }}

# Run the instrumented suite (pinned nightly, --branch) and hold kernel branch coverage to the pin.
coverage-kernel-branch:
    ./tools/covcheck --self-test
    @mkdir -p target/llvm-cov
    cargo +{{ nightly_toolchain }} llvm-cov nextest --branch --workspace --all-features --lcov --output-path target/llvm-cov/workspace-branch.lcov
    ./tools/covcheck --branch --lcov target/llvm-cov/workspace-branch.lcov --expected {{ covcheck_expected }} --root {{ justfile_directory() }}
