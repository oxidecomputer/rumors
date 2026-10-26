# rumors workspace: the source of truth for verification. Every artifact in
# the workspace has a recipe here, tiered by feedback speed, and `just --list`
# is the tour. The one exception class is the hand-run instrument: an
# `#[ignore]`-gated test no recipe runs, whose own doc states the cost that
# keeps it out and the command that runs it.
#
#   inner loop   just check / just test <filter>     seconds to a minute
#   commit gate  just gate                           fully clean before every commit
#   CI sweep     just ci                              the CI build job
#   full sweep   just all                             CI plus developer-only sweep checks
#
# The gate runs every check a commit must pass: build-free lints first,
# then every building leg concurrently (see the comment above `gate` for
# the stream grouping and why parallelism cannot move a verdict).
# `ci` is the build-everything job GitHub runs. The other `ci-*` recipes own
# its instrument and high-count-property jobs. `all` composes every CI group
# with the checks that require a developer machine (`local-only`). Each check
# is listed once below; the workflow invokes the owning group rather than
# copying its contents.
#
# Tools: rust-toolchain.toml provisions the stable toolchain with its
# components and targets on first use. Across tiers the recipes also use
# cargo-nextest, python3 (the tools/ linters), and the nightly pinned at
# `nightly_toolchain`; a recipe that needs anything else names it in its
# comment.

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
# by two others (the fuzz-fit harness and the fuelscape pipeline).
#
# Named here, and passed explicitly at the producer and at every consumer,
# so an ambient `CARGO_TARGET_DIR` cannot separate the guest from the
# consumers that read it: every tool recipe passes `--target-dir`, which
# overrides the variable. The guest builds in the root manifest's `fuel`
# profile, which is part of what fuel measures. The tool recipes keep
# target directories of their own under `target/`, so the gate's streams
# can overlap; the `gate` recipe's comment states when they do.

fuzzfit_target := justfile_directory() + "/target/fuzzfit"
fuzzfit_guest_wasm := fuzzfit_target + "/wasm32-unknown-unknown/fuel/fuzzfit_guest.wasm"

# The fuelscape atlas's and the surface-totality checker's target
# directories, kept apart from the root `target/` for the same reason.

fuelscape_target := justfile_directory() + "/target/before-fuelscape"
surfacecheck_target := justfile_directory() + "/target/surfacecheck"

# The 32-bit boundary-pin guest's wasm: produced by `wasm32-pins-build` in
# the root manifest's `wasm32-pins` profile, read by the `wasm32-pins`
# harness run. Named and passed explicitly for the same reason as the
# fuzz-fit guest above: an ambient CARGO_TARGET_DIR must not separate the
# producer from the consumer.

wasm32pins_target := justfile_directory() + "/target/wasm32-pins"
wasm32pins_guest_wasm := wasm32pins_target + "/wasm32-unknown-unknown/wasm32-pins/wasm32_pins_guest.wasm"

# The fuel board's guest wasm: produced by `amp-board-fuel-build` in the
# `fuel` profile, read by the fuel board's harness. Named and passed
# explicitly for the same reason as the fuzz-fit guest above.

board_fuel_target := justfile_directory() + "/target/board-fuel"
board_fuel_guest_wasm := board_fuel_target + "/wasm32-unknown-unknown/fuel/board_fuel_guest.wasm"

# The focused-fuel guest's wasm: produced by `focused-fuel-build` in the
# `fuel` profile, read by the focused-fuel harness. Named and passed
# explicitly for the same reason as the fuzz-fit guest above.

focused_fuel_target := justfile_directory() + "/target/focused-fuel"
focused_fuel_guest_wasm := focused_fuel_target + "/wasm32-unknown-unknown/fuel/focused_fuel_guest.wasm"

# The environment every fuel-metered wasm guest builds under. A guest embeds
# source paths (panic locations), and their lengths shift its memory layout
# and so its fuel readings. Paths inside this workspace are already
# relative, so the host-specific prefixes are those of dependency sources
# under CARGO_HOME and of the toolchain's rust-src. These flags map both to
# fixed ones, and clear any rustc wrapper, which could add mappings of its
# own, so every host builds the same module. The value is shell text that a
# recipe puts in front of its cargo command, expanded when the line runs.

fuel_guest_env := "RUSTC_WRAPPER= RUSTFLAGS=\"--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo --remap-path-prefix=$(rustc --print sysroot)/lib/rustlib/src/rust=/rustc/$(rustc -vV | sed -n 's/^commit-hash: //p')\""

# The interleaving-fuel guest's wasm: produced by `suanpan-fuel-build` in
# the `fuel` profile, read by its harness. Named and passed explicitly for
# the same reason as the fuzz-fit guest above.

suanpan_fuel_target := justfile_directory() + "/target/suanpan-fuel"
suanpan_fuel_guest_wasm := suanpan_fuel_target + "/wasm32-unknown-unknown/fuel/suanpan_fuel_guest.wasm"

# List recipes.
default:
    @"{{ just_executable() }}" --justfile "{{ justfile() }}" --list --unsorted

# ── inner loop ───────────────────────────────────────────────────────────────

# Type-check every host target: libs, tests, benches, examples. A target
# with `required-features` (the amplification board example) is skipped here
# and type-checked by the gate's all-features legs instead.
check:
    cargo check --all-targets

# Run the test suites; pass a filter to narrow (`just test mirror`).
test *args:
    cargo nextest run {{ args }}

# Every feature is lit here and nowhere else in the gate.

# Run the test suites under every feature (the gate's test run).
test-all *args:
    cargo nextest run --all-features --no-fail-fast {{ args }}

# Run Rumors at a larger property-test count under the faster release profile.
# This stays package-scoped: before's operation meters intentionally pin the
# dev profile and have their own exhaustive verification.
test-release *args:
    PROPTEST_CASES={{ proptest_ci_cases }} cargo nextest run -p rumors --all-features --locked --cargo-profile release --profile high-count --no-fail-fast {{ args }}

# Stable rustdoc compiles one executable per example; `before` has nearly 100,
# and their macOS link work dominates the gate. Nightly's merged mode compiles
# one harness per crate instead. Keep its target separate so switching compilers
# cannot invalidate the stable gate artifacts (or vice versa).

# Run the doctests (nightly), which nextest does not run.
doctest:
    RUSTDOCFLAGS="-Z unstable-options --merge-doctests yes" cargo +{{ nightly_toolchain }} test --doc --all-features --target-dir target/doctest-nightly

# Lint every target, warnings denied (the commit-gate setting).
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

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
# reporting), so the tool reads the member manifests cargo names. It also
# holds the default members, which divide production crates from tools, and
# nextest's default filter, which keeps the tools' tests out of a plain
# nextest run, to each other and to the dependency graph: cargo cannot tell
# a tool from any other member. The tool's header lists what the rules check.
# Build-free, so it rides the lint tier.

# Hold member manifests to the workspace table, and the tool-side members to the default members and nextest's default filter.
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
    RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
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
    RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --document-private-items --target-dir target/doc-internal
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
# Needs cargo-docs-rs: `cargo install cargo-docs-rs`.

# Build the rustdoc as docs.rs does (pinned nightly, `--cfg docsrs`, each crate's docs.rs metadata).
docs-docsrs:
    RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +{{ nightly_toolchain }} doc -p rumors --all-features --no-deps --target-dir target/doc-docsrs
    cargo +{{ nightly_toolchain }} docs-rs -p rumors --target-dir target/doc-docsrs
    cargo +{{ nightly_toolchain }} docs-rs -p before --target-dir target/doc-docsrs
    ./tools/fuelscape-assets target/doc-docsrs/{{ host_triple }}/doc

# tests/future_size.rs pins the public futures' sizes, and a `cfg` that
# compiles it empty would read as a pass. This leg reruns that binary from
# test-all's build set (same features, so no rebuild) with
# `--no-tests=fail`, so an empty binary fails the gate. The filter names
# the binary by its package-scoped id, so a same-named binary in another
# member cannot keep the leg green while rumors' collects nothing.

# Fail unless the future-size pins were collected and pass (liveness for tests/future_size.rs).
future-size:
    cargo nextest run --all-features -E 'binary_id(rumors::future_size)' --no-tests=fail

# The amplification board's smoke suite (crates/before/tests/amp_board_smoke.rs)
# drives every board cell at a small scale. It is a resource instrument, so
# nextest's default filter keeps it out of a plain run, and this recipe is
# how the gate reaches it. It shares test-all's selection and features, so
# it reuses test-all's build.

# Run the amplification board's smoke suite, which a plain nextest run skips.
amp-board-smoke:
    cargo nextest run --all-features --ignore-default-filter -E 'binary_id(before::amp_board_smoke)' --no-tests=fail

# The supply-chain leg, two build-free checks over the committed lockfiles.
# cargo-audit sweeps every lockfile tracked by Git — the root workspace's,
# which every member shares, tool-side ones included, and the detached fuzz
# workspace's — against the RustSec advisory database (fetched once over
# the network, so this is the one gate leg that needs connectivity): a
# vulnerability anywhere fails the gate; unmaintained/unsound/yanked
# advisories print as warnings for triage without failing. cargo-deny holds
# the resolved graph of the root workspace's default members (all features,
# all targets) to one version per crate; deny.toml is the roster of record,
# every tolerated duplicate carrying the holdout that keeps it alive. The duplicate policy leaves the tool-side members out: a
# duplicate only they pull in costs one extra tool compile, never a
# shipped byte. The root manifest is a package AND a workspace, and
# cargo-deny without `--workspace` would check the root package's graph
# alone, missing any duplicate only another member pulls in.
# Needs cargo-audit and cargo-deny: `cargo install cargo-audit cargo-deny`.

# Audit advisories on every lockfile and hold the workspace to single crate versions.
supply-chain:
    #!/usr/bin/env bash
    set -euo pipefail
    fetch=1
    while IFS= read -r -d '' lock; do
        if ((fetch)); then
            cargo audit --file "$lock"
            fetch=0
        else
            cargo audit --no-fetch --file "$lock"
        fi
    done < <(git ls-files -z -- '*Cargo.lock')
    # cargo-deny takes the root package alone, or every member with
    # `--workspace`, and never reads default-members, so the tool-side
    # members (every non-default member) are excluded by name.
    tools=$(cargo metadata --no-deps --format-version 1 | python3 -c '
    import json, sys
    m = json.load(sys.stdin)
    names = {p["id"]: p["name"] for p in m["packages"]}
    defaults = set(m["workspace_default_members"])
    print(" ".join(f"--exclude {names[i]}" for i in m["workspace_members"] if i not in defaults))
    ')
    # Word splitting is wanted: `tools` is a list of `--exclude NAME` pairs.
    # shellcheck disable=SC2086
    cargo deny --workspace $tools check bans

# The fuzz targets live in a detached workspace (crates/before/fuzz), so no
# root-workspace command formats, lints, tests, or compiles them. This gate leg
# provides that ordinary Rust verification. Its library tests replay every
# committed seed through the same target body and heap guard; the short
# coverage-guided libFuzzer run remains a `just all` sweep because it is too
# expensive and nondeterministic for per-commit judgment.
# Needs cargo-fuzz: `cargo install cargo-fuzz`.

# Verify the detached fuzz crate against its committed lockfile and compile
# every libFuzzer target (nightly).
[working-directory("crates/before/fuzz")]
fuzz-build:
    cargo fmt --check
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked --lib
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
# target directory serialize on its build lock, so every leg that builds in
# the root `target/` sits in one stream and runs in order there,
# cheap-first, preserving the fail-fast ordering within it. Every other leg
# builds nothing or builds in a target directory of its own, passed with
# `--target-dir` or belonging to the detached fuzz workspace, and that is
# what lets them overlap. `fuzzfit` and `fuelscape-test` share one stream
# because both build the same wasm guest.
#
# That holds where each target directory is also its own build directory,
# cargo's default. A `build.build-dir` set in a cargo config gives every
# invocation in a workspace one build directory, and one build lock,
# whatever its target directory, so there the streams' builds wait on one
# another and overlap less. The verdicts are the same either way; only the
# wall time grows.
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
        recipe=$3
        began=$SECONDS
        echo "===== just $recipe ====="
        # The marker is the record and the echo is narration, so the marker
        # is written first: a stream that reported ok has already banked it.
        # The stream re-invokes the running just on this justfile, so it
        # runs the same binary and file however the gate was started, with
        # `-f`, `-d`, or another `just` first on PATH. Variable overrides
        # given on the gate's command line do not reach the streams.
        if nice -n "$niceness" "{{ just_executable() }}" --justfile "{{ justfile() }}" "$recipe"; then
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
        recipe=$3
        streams+=("$name")
        run_stream "$name" "$niceness" "$recipe" > "$logs/$name.log" 2>&1 &
        echo "gate: start   $name ($recipe)"
    }

    # The workspace stream carries the test suite and is the critical
    # path; every other stream together is less work than it is, and all
    # of them finish first even when yielding. So they run niced: the
    # tests keep first call on the cores, and the shorter streams fill
    # what the tests leave idle instead of competing for it.
    began=$SECONDS
    start_stream workspace     0 _gate-workspace
    start_stream doctest      10 _gate-doctest
    start_stream board        10 _gate-board
    start_stream board-fuel   10 _gate-board-fuel
    start_stream suanpan-fuel 10 _gate-suanpan-fuel
    start_stream focused-fuel 10 _gate-focused-fuel
    start_stream wasm         10 _gate-wasm
    start_stream fuzz         10 _gate-fuzz
    start_stream surface      10 _gate-surface
    start_stream internal-docs 10 _gate-internal-docs
    start_stream docsrs       10 _gate-docsrs
    start_stream audit        10 _gate-audit
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

# The stream recipes are the gate's build roster. Keeping each stream's legs
# here lets the gate runner choose scheduling without becoming a second list of
# checks; CI reuses the same coherent groups where its cadence overlaps.
_gate-workspace: clippy clippy-default docs test-all future-size amp-board-smoke
_gate-doctest: doctest

# One sweep judges each cell and which family reads worst; the
# acceptance recipe's comment describes both judgments.
_gate-board: amp-board-acceptance

# The same cells judged in Wasmtime fuel; the fuel recipe's comment says why.
_gate-board-fuel: amp-board-fuel

# suanpan's cost rules judged over random interleavings, in fuel.
_gate-suanpan-fuel: suanpan-fuel

# The cost claims the board cannot state, judged in fuel.
_gate-focused-fuel: focused-fuel

_gate-wasm: fuzzfit fuelscape-test wasm32-pins
_gate-fuzz: fuzz-build
_gate-surface: surface-totality
_gate-internal-docs: docs-internal
_gate-docsrs: docs-docsrs
_gate-audit: supply-chain

# ── artifacts the gate doesn't reach ─────────────────────────────────────────
# `serde` is compiled in every build that includes `rumors`, which enables it,
# and `oracle` in every `before` test build, through `before`'s dev-dependency
# on itself. Only the wasm32-pins guest enables `borsh`, together with
# `meter`, so outside `--all-features` builds only this recipe compiles
# `borsh` alone or with only `serde`. The `serde`+`borsh` pair matters
# because both derive on the same types.

# Feature matrix: every cfg-gated surface on its own, so nothing rots behind `--all-features`.
features:
    cargo check -p suanpan --no-default-features
    cargo check -p before --no-default-features
    cargo check -p before --no-default-features --features serde
    cargo check -p before --no-default-features --features borsh
    cargo check -p before --no-default-features --features oracle
    cargo check -p before --no-default-features --features meter
    cargo check -p before --no-default-features --features laws
    cargo check -p before --no-default-features --features serde,borsh
    cargo check -p rumors --no-default-features
    cargo check -p rumors --no-default-features --features fs
    cargo check -p rumors --no-default-features --features conformance

# The viz engine must keep compiling for its real target, not just the host.
wasm-check:
    cargo check -p before-viz --target wasm32-unknown-unknown

# This is exactly what the Pages deploy runs. Needs wasm-pack and npm
# (network on first run).

# Full visualizer build: wasm-pack, strict TypeScript typecheck, esbuild bundle.
viz:
    ./crates/before-viz/build.sh

# The only build that exercises the bench/release profile; benches otherwise rot silently.

# Compile (don't run) the criterion benches.
bench-build:
    cargo bench --no-run

# The decode invariant (accepted input re-encodes stably and decodes back to
# itself) and the `before::testing::laws` law collection are asserted inline in
# the targets, so any hit is a crash. Each run names two corpus directories:
# libFuzzer reads seeds from both and writes new discoveries to the first, so
# the committed `seeds/<target>/` corpus actually seeds every run while staying
# pristine. `cargo fuzz list` is the roster of record: a new target cannot be
# compiled by the gate yet silently omitted from this sweep.

# Short fuzz smoke: run each libFuzzer target for `secs` seconds.
[working-directory("crates/before/fuzz")]
fuzz secs=fuzz_smoke_secs:
    #!/usr/bin/env bash
    set -euo pipefail
    while IFS= read -r target; do
        # libFuzzer requires its write corpus to exist. That discovery corpus
        # is deliberately untracked, while the committed seed corpus remains
        # pristine as the run's second input.
        mkdir -p "corpus/$target"
        cargo +{{ nightly_toolchain }} fuzz run --target {{ host_triple }} "$target" "corpus/$target" "seeds/$target" -- -max_total_time={{ secs }}
    done < <(cargo +{{ nightly_toolchain }} fuzz list)

# The fuzz-fit asymptotics harness (crates/before/fuzzfit) is a pair of
# tool-side workspace members. Neither a plain cargo command at the root nor
# the justfile's workspace-wide recipes build them, since they are not
# default members, so the ordinary legs never compile them or wasmtime. A
# bare `cargo <cmd> --workspace` does compile both, and nextest's default
# filter keeps their tests out of such a run. The gate reaches them only
# through these recipes by name.
# The guest compiles selected public operations to wasm32-unknown-unknown in
# the `fuel` profile, built alone so its features are exactly its own and
# under `fuel_guest_env`, so every host builds the same module; the
# harness replays generated programs natively and under wasmtime fuel
# metering (deterministic instruction counts, byte-reproducible under any
# load) and judges each measured step against its pinned fuel band in
# harness/src/bands.rs. A cost change in that vocabulary fails here and must
# be re-pinned deliberately (`just fuzzfit-calibrate`) instead of drifting.

# Build the fuzz-fit wasm guest and its harness (both halves).
fuzzfit-build:
    {{ fuel_guest_env }} cargo build -p fuzzfit-guest --profile fuel --target wasm32-unknown-unknown --target-dir {{ fuzzfit_target }}
    cargo build -p fuzzfit-harness --tests --release --target-dir {{ fuzzfit_target }}

# Run the fuzz-fit suites: generator sanity, meter liveness, the judgment
# and shape-leg tripwires, the quadratic-burner adequacy check, the
# toolchain-pin and staleness cross-checks, and the enforcement check
# (48 generated programs against the pinned bands, point and trend checks,
# plus the whole 256-program deterministic prefix judged step by step).
# A failure shrinks to a minimal out-of-band shape and writes a proptest
# seed file -- commit any seed that appears. The clippy line is the
# members' own lint leg, since the workspace-wide clippy leaves them out (the
# root `cargo fmt --all` reaches them); every tool recipe that runs tests
# carries the same lint leg.

# Run the fuzz-fit asymptotics suites against the pinned fuel bands.
fuzzfit: fuzzfit-build
    cargo clippy -p fuzzfit-guest -p fuzzfit-harness --all-targets --target-dir {{ fuzzfit_target }} -- -D warnings
    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo nextest run --ignore-default-filter -p fuzzfit-guest -p fuzzfit-harness --cargo-profile release --target-dir {{ fuzzfit_target }}

# Re-fit the pinned bands from the committed deterministic corpus (4096
# programs; byte-reproducible, so any diff is a real change). Rewrites
# harness/src/bands.rs atomically: review the diff like a snapshot and
# commit with the movement annotated in the commit message.

# Re-fit and rewrite the fuzz-fit harness's pinned fuel bands.
fuzzfit-calibrate: fuzzfit-build
    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo run --release -p fuzzfit-harness --bin calibrate --target-dir {{ fuzzfit_target }}

# The 32-bit boundary pins (crates/before/wasm32-pins, tool-side workspace
# members) execute `before` at the 32-bit boundaries where `usize`
# conversions or backend capacity can change behavior. `just wasm-check`
# proves only that `before` compiles for a 32-bit target; these pins run the
# operations, on inputs synthesized inside wasm so a 64-bit host cannot
# conceal the boundary. Each boundary has adjacent cases so a failure
# identifies the exact transition. The guest builds in the root manifest's
# `wasm32-pins` profile, whose overflow checks are off as in an ordinary
# release build, so explicit checks must prevent a 32-bit wrap from
# changing the observed result.

# Build the 32-bit boundary-pin wasm guest and its harness (both halves).
wasm32-pins-build:
    cargo build -p wasm32-pins-guest --profile wasm32-pins --target wasm32-unknown-unknown --target-dir {{ wasm32pins_target }}
    cargo build -p wasm32-pins-harness --tests --release --target-dir {{ wasm32pins_target }}

# The deep pins walk hundreds of megabytes inside a 32-bit guest, so the
# run costs minutes of wall time and peaks at a few GiB of host memory
# across nextest's parallel workers. The clippy line is the members' own
# lint leg, since the workspace-wide clippy leaves them out.

# Run the 32-bit boundary pins under wasmtime (minutes; a few GiB of host memory).
wasm32-pins: wasm32-pins-build
    cargo clippy -p wasm32-pins-guest -p wasm32-pins-harness -p wasm32-pins-protocol --all-targets --target-dir {{ wasm32pins_target }} -- -D warnings
    WASM32_PINS_GUEST_WASM={{ wasm32pins_guest_wasm }} cargo nextest run --ignore-default-filter -p wasm32-pins-guest -p wasm32-pins-harness -p wasm32-pins-protocol --cargo-profile release --target-dir {{ wasm32pins_target }}

# The interleaving-fuel property (crates/suanpan/fuel, tool-side workspace
# members) generates programs over a pool of suanpan accumulators, runs each
# in a wasm32 guest under Wasmtime fuel, and checks that fuel grows only as
# suanpan's cost rules allow when every size in the program grows 64- and
# 4096-fold and when the program repeats 4096 times. Every reading is judged
# against another reading of the same program, so nothing here is
# calibrated, and no toolchain or Wasmtime bump needs re-pinning; the
# harness's crate docs state the bounds and what they cannot see. The guest
# builds in the `fuel` profile and under `fuel_guest_env`, so every host
# builds the same module, and shares the fuel board's window allocator and
# operator prices. A failure shrinks to a minimal skeleton and writes a
# proptest seed file: commit any seed that appears. The clippy and rustdoc
# lines are the members' own lint and docs legs, since the workspace-wide
# ones leave them out; the guest's run for wasm32, the only target it builds
# for. The rustdoc lines build the public and the private-items docs, each
# with warnings denied, because each catches broken links the other cannot.

# Build the interleaving-fuel wasm guest and its harness (both halves).
suanpan-fuel-build:
    {{ fuel_guest_env }} cargo build -p suanpan-fuel-guest --profile fuel --target wasm32-unknown-unknown --target-dir {{ suanpan_fuel_target }}
    cargo build -p suanpan-fuel-harness -p suanpan-fuel-protocol --tests --release --target-dir {{ suanpan_fuel_target }}

# Run suanpan's interleaving-fuel property, the census of its skeletons, and
# the focused cost claims judged on fixed programs.
suanpan-fuel: suanpan-fuel-build
    cargo clippy -p suanpan-fuel-guest --target wasm32-unknown-unknown --target-dir {{ suanpan_fuel_target }} -- -D warnings
    cargo clippy -p suanpan-fuel-harness -p suanpan-fuel-protocol --all-targets --target-dir {{ suanpan_fuel_target }} -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc -p suanpan-fuel-harness -p suanpan-fuel-protocol --no-deps --target-dir {{ suanpan_fuel_target }}/doc-public
    RUSTDOCFLAGS="-D warnings" cargo doc -p suanpan-fuel-harness -p suanpan-fuel-protocol --no-deps --document-private-items --target-dir {{ suanpan_fuel_target }}/doc-internal
    RUSTDOCFLAGS="-D warnings" cargo doc -p suanpan-fuel-guest --target wasm32-unknown-unknown --no-deps --target-dir {{ suanpan_fuel_target }}/doc-public
    RUSTDOCFLAGS="-D warnings" cargo doc -p suanpan-fuel-guest --target wasm32-unknown-unknown --no-deps --document-private-items --target-dir {{ suanpan_fuel_target }}/doc-internal
    SUANPAN_FUEL_GUEST_WASM={{ suanpan_fuel_guest_wasm }} cargo nextest run --ignore-default-filter -p suanpan-fuel-harness -p suanpan-fuel-protocol --cargo-profile release --target-dir {{ suanpan_fuel_target }}

# The population atlas (crates/before-fuelscape) is a tool-side workspace
# member like the fuzz-fit crates: neither plain cargo at the root nor the
# justfile's workspace-wide recipes build it, so the ordinary legs never
# compile it or its wasmtime and plotters tooling. The gate reaches it
# only through these recipes by name. Its committed tests are the sampler
# adequacy pins — counting tables against exhaustive grammar enumeration and
# the real decoders' accept sets, chi-square uniformity, codec round-trips —
# plus a tiny end-to-end pipeline smoke (sample, measure fuel in the
# fuzz-fit guest, render), which is why the guest builds first. Audit-only
# by design: nothing here enforces a fuel number — the amplification board
# and the fuzz-fit bands own enforcement.

# Lint and test the fuelscape: sampler adequacy pins plus the pipeline smoke.
fuelscape-test: fuzzfit-build
    cargo clippy -p before-fuelscape --all-targets --target-dir {{ fuelscape_target }} -- -D warnings
    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo nextest run --ignore-default-filter -p before-fuelscape --target-dir {{ fuelscape_target }}

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
    FUELSCAPE_TIP=$(git rev-parse HEAD) FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo run --release -p before-fuelscape --bin fuelscape --target-dir {{ fuelscape_target }} -- --out {{ justfile_directory() }}/target/fuelscape {{ args }}

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
    cargo run --release -p before-fuelscape --bin fuelscape --target-dir {{ fuelscape_target }} -- --compact-from dump --out {{ justfile_directory() }}/crates/before/fuelscape

# Verify the committed widget datasets equal a fresh compaction of the committed dump.
[working-directory("crates/before-fuelscape")]
fuelscape-verify:
    rm -rf {{ justfile_directory() }}/target/fuelscape-verify
    cargo run --release -p before-fuelscape --bin fuelscape --target-dir {{ fuelscape_target }} -- --compact-from dump --out {{ justfile_directory() }}/target/fuelscape-verify
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

# Run one bench target through a Criterion filter, at full sampling. Additional
# arguments pass through to Criterion. For example,
# `just bench version version/hole --profile-time 10` records a focused
# profiler workload without a second profiling harness.
bench target *filter:
    cargo bench --bench {{ target }} -- {{ filter }}

# The reduced-sampling inner loop: `bench` at 10 samples x 1 s. Never quoted.
bench-quick target *filter:
    cargo bench --bench {{ target }} -- --sample-size 10 --measurement-time 1 {{ filter }}

# Each board cell judges a deterministic counter, peak heap, against its
# declared bounds: green means the samples fit, red is a resource regression
# to investigate. The board reads no clock, so its output is byte-identical
# under any machine load.
#
# The board runs at the release profile, the profile of record: debug
# assertions run verification scaffolding, so a dev board measures algorithm
# plus that scaffolding while release measures the production work alone. A dev run (`cargo run -p before --example amp_board ...`) remains
# a legitimate debugging view; its numbers must never be pinned anywhere.
#
# Every mode that sweeps (all but `compare`, which reads two captures)
# parallelizes by process sharding: the peak-heap column reads
# the process-global allocator, so the sweep stays single-threaded inside
# each process and the runner spawns children that split the operation x
# family cell grid instead, merging the measured samples back in board
# order. Every judged quantity is a deterministic counter read from a
# per-process allocator, so a reading is a function of its cell alone: neither machine load nor the shard layout
# that measured it can move one. AMP_BOARD_SHARDS overrides the count.
#
# A bare run (any single scale) is a debugging view whose verdicts never
# bind; the verdict of record is the acceptance invocation below, which
# measures each cell's whole ladder once. Optional scale multiplies the
# input sizes, e.g. `just amp-board 4`. The same recipe runs
# `just amp-board compare <before> <after>`, which compares two captures'
# exact readings (the amp_board example's doc describes it).

# The board's target directory. The board builds in the release profile,
# which no leg in the gate's workspace stream builds, so a directory of its
# own costs the gate no extra compilation, and the board stream and the
# workspace stream never wait on each other's build lock.
amp_board_target := justfile_directory() + "/target/amp-board"

# One command serves every board recipe, keeping the production profile and
# the target directory part of the instrument rather than conventions each
# entry point must repeat.
amp_board_command := "cargo run --release -p before --example amp_board --features serde,borsh --target-dir " + amp_board_target

# A board capture names the source tree it measured, so a calibration can
# refuse to merge readings of another tree. This command prints that name:
# the git tree of the checkout's tracked files as they stand, edits included
# and untracked files excluded. It stages the tracked files into a temporary
# copy of the index and writes that copy's tree, so it never takes
# `.git/index.lock` and leaves the checkout's index untouched; the copy keeps
# the index's stat cache, so only edited files are hashed. If the command
# fails, the recipe stops before measuring anything. The recipes below
# compute the name only for a run that captures or calibrates, and pass it to
# the runner as AMP_BOARD_SOURCE_TREE; a capture with no name is refused.
source_tree := 'd=$(mktemp -d) && trap "rm -rf \"$d\"" EXIT && cp "$(git rev-parse --git-path index)" "$d/index" && GIT_INDEX_FILE="$d/index" git add -u && GIT_INDEX_FILE="$d/index" git write-tree'

# Render the board at one scale (a debugging view), or `compare` two captures.
[positional-arguments]
amp-board *args:
    {{ amp_board_command }} -- "$@"

# The board's one verdict of record. One invocation sweeps each cell's
# whole ladder once — two sizes at each of the two sampling scales
# (board::DEFAULT_SCALE and board::LADDER_TOP_SCALE), plus two pairs for
# ceilings and liveness alone: small inputs, and size knobs just past a
# power of two (board::Sampling::PastPowerOfTwo), where a buffer that grows
# by doubling holds its peak — and judges that sweep twice:
#
# - Acceptance judges every constant and floor per size and fits each
#   exponent as one trend across the ladder. A red cell is an untriaged
#   contradiction, resolved only by a cure or an owner-declared model at
#   the cell.
# - The worst-case ranking pin (the WORST_RANKINGS table beside the fold)
#   is entry-compared against the live argmax family per operation x
#   currency at both sampling scales, so a ranking flip is caught here,
#   never discovered by a reader. A flip is news: either a family
#   legitimately overtook (re-pin deliberately with a movement annotation)
#   or a code change made some shape relatively worse (investigate first).
#   A cell is declared TARGET_DEPENDENT in the table, and then needs only a
#   live worst set, only when a target-conditional code path the measured
#   operation reaches changes which family reads worst between the 64-bit
#   hosts the pin is checked on; the entry's comment names that path. A
#   flip with no such path behind it, however near the tie, never
#   qualifies: investigate it or re-pin it as above.
#
# Both judgments run whatever the other's verdict, so ranking drift is
# attributed to the commit that causes it. Red rows and drift lines print
# to stdout; stderr carries a one-line summary of each failed judgment. The
# run exits nonzero on any red cell or any drift, so the gate's board
# stream fails on either. Every judged quantity is a deterministic
# counter, so a second run reads the same board and needs no repeated hand
# runs.
#
# `--capture <file>` also writes the sweep's exact readings to <file>,
# whatever the verdict, stamped with the source tree, for a later `just
# amp-board compare`: take one capture before a change and one after, and the
# developer loop stays one board run per iteration.

# Run the board's verdict of record: one ladder sweep, acceptance and the ranking pin.
[positional-arguments]
amp-board-acceptance *args:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ ${1-} == --capture ]]; then
        tree="$({{ source_tree }})"
        export AMP_BOARD_SOURCE_TREE="$tree"
    fi
    exec {{ amp_board_command }} -- acceptance "$@"

# The fuel board runs every board cell inside a wasm32 guest under Wasmtime
# fuel metering and judges fuel, a count of every wasm instruction the cell
# body executes, by the board's own acceptance rules: the exponent bound,
# declared models, and a ceiling calibrated per cell. It is the board's
# measure of execution time, element moves inside a `Vec`, arithmetic inside
# `num-bigint`, and buffer fills included. The native board above stays the
# measure of record for heap.
#
# Fuel is deterministic for a given guest build and cost table, so the
# verdict is unaffected by machine load and by the worker count
# (AMP_BOARD_FUEL_THREADS): the guest gives each measured body a fresh heap,
# so a reading never depends on what the cell's preparation left in the
# heap, and every cell runs in its own fresh guest instance, so no other
# state an earlier cell left reaches it. The harness's window test, run here
# before the judgment, checks the first property on representative cells.
# The ceilings record the compiler (read from the guest's own `producers`
# section), Wasmtime version, and cost table they were calibrated under, and
# the run refuses to judge a guest built under any other, or one that embeds
# a path under the home directory; after a toolchain or Wasmtime bump,
# re-derive them with `just amp-board-calibrate-fuel`. The members' clippy
# runs here, since the workspace-wide clippy leaves them out; the guest's runs
# for wasm32, the only target it builds for.
#
# `--capture <file>` also writes the sweep's exact fuel readings, which
# `just amp-board compare` compares like any board capture and
# `just amp-board-calibrate-fuel <file>` recalibrates the fuel table from.

# The guest builds under `fuel_guest_env`, which fixes the source paths it
# embeds, so every host builds the same module.

# Build the fuel board's wasm guest and its harness (both halves).
amp-board-fuel-build:
    {{ fuel_guest_env }} cargo build -p board-fuel-guest --profile fuel --target wasm32-unknown-unknown --target-dir {{ board_fuel_target }}
    cargo build -p board-fuel-harness --release --target-dir {{ board_fuel_target }}

# Run the fuel board's verdict of record: every cell judged in Wasmtime fuel.
[positional-arguments]
amp-board-fuel *args: amp-board-fuel-build
    #!/usr/bin/env bash
    set -euo pipefail
    cargo clippy -p board-fuel-guest --target wasm32-unknown-unknown --target-dir {{ board_fuel_target }} -- -D warnings
    cargo clippy -p board-fuel-harness --all-targets --target-dir {{ board_fuel_target }} -- -D warnings
    BOARD_FUEL_GUEST_WASM={{ board_fuel_guest_wasm }} cargo nextest run --ignore-default-filter --release -p board-fuel-harness --target-dir {{ board_fuel_target }}
    if [[ ${1-} == --capture ]]; then
        tree="$({{ source_tree }})"
        export AMP_BOARD_SOURCE_TREE="$tree"
    fi
    BOARD_FUEL_GUEST_WASM={{ board_fuel_guest_wasm }} exec cargo run --release -p board-fuel-harness --target-dir {{ board_fuel_target }} -- acceptance "$@"

# Every board cell carries per-cell ceilings derived from its own readings
# (the board's `cell_ceilings` module states the rule): one heap table and
# one fuel table. Heap readings differ between the heap table's two
# architectures of record (aarch64 and x86_64, `board::HEAP_ARCHS`), and the
# table holds on both because each cell's ceiling is derived from the larger
# of its two readings at every sample. A re-derivation takes two hosts and
# one tree:
#
# 1. Bring both hosts to the same source tree (commit, or sync the edits).
# 2. On the host of one architecture, capture its readings:
#    `just amp-board-acceptance --capture <file>`.
# 3. On the host of the other, `just amp-board-calibrate <file>`, with an
#    absolute path. It sweeps this host's board without judging, refuses the
#    capture unless it names this source tree and, with this host, covers
#    exactly the architectures of record, takes each sample's larger heap
#    reading, and rewrites the heap table. Then it runs
#    `amp-board-calibrate-fuel`. Every cell whose ceiling moved prints.
#
# Fuel readings do not depend on the host, so a change that moves fuel alone
# needs only the fuel table rewritten, on one host, by
# `just amp-board-calibrate-fuel`:
#
# - After `just amp-board-fuel --capture <file>` has shown the movement,
#   `just amp-board-calibrate-fuel <file>`, with an absolute path, derives
#   the table from that capture by the same rule, without sweeping again. It
#   refuses a capture of another source tree or build, and a guest built
#   under any provenance but the table's.
# - After a toolchain or Wasmtime bump, or a cost-table change, the fuel
#   board refuses the guest and writes no capture, so run
#   `just amp-board-calibrate-fuel` with no capture: it sweeps the fuel board
#   and records the new provenance.
#
# Review the rewritten tables like snapshots, quoting the movement in the
# commit message.

# Re-derive the per-cell heap and fuel ceilings from this host and another architecture's capture.
[positional-arguments]
amp-board-calibrate *captures: && amp-board-calibrate-fuel
    #!/usr/bin/env bash
    set -euo pipefail
    tree="$({{ source_tree }})"
    AMP_BOARD_SOURCE_TREE="$tree" {{ amp_board_command }} -- calibrate "$@"

# Re-derive the per-cell fuel ceilings alone, from a capture of this tree's fuel board or a fresh fuel sweep.
[positional-arguments]
amp-board-calibrate-fuel *capture: amp-board-fuel-build
    #!/usr/bin/env bash
    set -euo pipefail
    tree="$({{ source_tree }})"
    AMP_BOARD_SOURCE_TREE="$tree" BOARD_FUEL_GUEST_WASM={{ board_fuel_guest_wasm }} cargo run --release -p board-fuel-harness --target-dir {{ board_fuel_target }} -- calibrate "$@"

# The focused-fuel suites state the cost claims the board cannot: that an
# operation's cost does not depend on a parameter, that one entry point
# costs another plus a fixed amount, that a fused walk costs less than the
# composition it replaces, or that a cost grows along an axis no board
# family scales. Each claim is a relation between readings of one guest, so
# no fuel number is committed and the suites need no recalibration after a
# toolchain or Wasmtime bump. The harness's crate docs name the forms.
# The guest builds under `fuel_guest_env`, as the board's does. The members'
# clippy runs here, since the workspace-wide clippy leaves them out.

# Build the focused-fuel wasm guest and its harness's tests.
focused-fuel-build:
    {{ fuel_guest_env }} cargo build -p focused-fuel-guest --profile fuel --target wasm32-unknown-unknown --target-dir {{ focused_fuel_target }}
    cargo build -p focused-fuel-harness -p focused-fuel-protocol --tests --release --target-dir {{ focused_fuel_target }}

# Run the cost claims the board cannot state, each judged in Wasmtime fuel.
focused-fuel *args: focused-fuel-build
    cargo clippy -p focused-fuel-guest --target wasm32-unknown-unknown --target-dir {{ focused_fuel_target }} -- -D warnings
    cargo clippy -p focused-fuel-harness -p focused-fuel-protocol --all-targets --target-dir {{ focused_fuel_target }} -- -D warnings
    FOCUSED_FUEL_GUEST_WASM={{ focused_fuel_guest_wasm }} cargo nextest run --ignore-default-filter -p focused-fuel-harness -p focused-fuel-protocol --cargo-profile release --target-dir {{ focused_fuel_target }} {{ args }}

# The surface-totality leg holds the resource board's operation inventory
# against nightly rustdoc JSON — the compiler's own account of the public
# surface. A public function or method added anywhere fails until it gains a
# board disposition or a named exception.
# Trait impls (operators, codecs, derives) and non-function items (associated
# consts and types, statics, macros) are pinned by the censuses in
# crates/before/surfacecheck/src/census.rs, reconciled both ways, so a new impl
# or item reads red until reviewed. Trait resource decisions remain grouped in
# the board's family entries. The checker is a tool-side workspace member,
# which the workspace-wide gate legs never select, so this recipe also runs
# its lints and unit tests.
#
# Toolchain coupling: rustdoc JSON is an unstable format, versioned by
# its `format_version` field, and the checker's `rustdoc-types`
# dependency is pinned exact to the release speaking the installed
# nightly's format. The checker refuses — loudly, naming both numbers —
# any document whose format_version differs, so a nightly bump can fail
# this recipe but can never make it silently wrong. After bumping the
# nightly: run this recipe, and if it reports a format mismatch, move
# the `rustdoc-types` pin in the root Cargo.toml's workspace table to
# the release whose FORMAT_VERSION matches the new nightly's output
# (the rustdoc-types changelog maps releases to formats), then re-run
# until green.

# Build the nightly rustdoc JSON the surface-totality check parses.
surface-json:
    cargo +{{ nightly_toolchain }} rustdoc -p before --lib --all-features --target-dir target/surface-json -- -Z unstable-options --output-format json

# Hold before's public surface total against resource coverage.
surface-totality: surface-json
    cargo clippy -p surfacecheck --all-targets --target-dir {{ surfacecheck_target }} -- -D warnings
    cargo nextest run --ignore-default-filter -p surfacecheck --target-dir {{ surfacecheck_target }}
    cargo run -q -p surfacecheck --target-dir {{ surfacecheck_target }} -- {{ justfile_directory() }}/target/surface-json/doc/before.json

# The worst-case map answers "which committed shape is worst for operation
# X" mechanically: for every operation x currency it takes the argmax over
# the family roster of the board's own normalized constants (each cell's
# reading over its own denominator of record), with the runner-up and the
# margin beside it. Scope: the maximum over the committed roster --
# the claim that this is the true worst case is carried by the rustdoc
# complexity sections and the asymptotics liveness pins, not by this
# table. Runs at release, the board's profile of record, one table per
# sampling scale of the ladder. Its rankings are pinned and checked by
# amp-board-acceptance; this view adds the runner-ups and margins.

# Render the worst-case map: the argmax family per operation x currency, one table per sampling scale.
worst-cases:
    {{ amp_board_command }} -- worst-cases

# ── the no-rot sweep ─────────────────────────────────────────────────────────
# Each recipe below owns one CI job. The workflow calls these groups without
# restating their members, while `all` composes the same groups for a local
# sweep. Gate-only instruments remain in `_gate-wasm`: their deterministic
# fuel and 32-bit checks run before commits but are intentionally absent from
# shared runners. `all` adds the fuzz smoke and formal checks that CI does not
# have the runtime or toolchain to execute.

# Build everything in CI's primary job, without running libFuzzer.
ci: gate-lints _gate-workspace _gate-doctest _gate-fuzz _gate-internal-docs _gate-docsrs fuelscape-claims features wasm-check bench-build fuelscape-verify viz

# Re-run the gate judgments suitable for shared CI runners.
ci-instruments: _gate-audit _gate-board _gate-surface

# Run the larger release-profile property population.
ci-properties: test-release

# Run the checks that require local fuzz and Lean toolchains.
local-only: (fuzz fuzz_smoke_secs) lean eventdag muxprobe

# Run every CI group and every local-only sweep check.
all: ci ci-instruments ci-properties local-only
