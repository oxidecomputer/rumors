# rumors workspace verification. `just --list` is the tour.
#
#   inner loop   just check / just test <filter>
#   commit gate  just gate    fully clean before every commit
#   CI           just ci      with ci-instruments and ci-properties, one per CI job
#   full sweep   just all     CI plus the local-only fuzz and Lean checks
#
# `#[ignore]`d instruments have no recipe; each one's doc gives its cost and
# the command that runs it.
#
# Tools: rust-toolchain.toml provisions stable. The recipes also need
# cargo-nextest, python3, and the nightly at `nightly_toolchain`; a recipe
# that needs anything else says so.

set shell := ["bash", "-euo", "pipefail", "-c"]

# Merged doctests, the fuzz build, and rustdoc JSON need nightly. It is dated
# because surfacecheck pins `rustdoc-types` exactly to the JSON
# `format_version` this nightly emits; a floating nightly would make
# surface-totality pass or fail on whatever each machine last downloaded.
#
# To bump: change the date, run `just surface-totality`, and if it reports a
# format mismatch, move the `rustdoc-types` pin in the root Cargo.toml to the
# release whose FORMAT_VERSION matches.

nightly_toolchain := "nightly-2026-06-30"

# Cases per property in CI's release-profile run. A divided property runs its
# share of this count (`before`'s `testing::property` module explains the
# division).
proptest_ci_cases := "4000"

# cargo-fuzz defaults `--target` to the triple it was built for, which for
# the static prebuilt CI installs is a musl target the sanitizer build
# cannot use. Naming the host avoids that.

host_triple := `rustc -vV | sed -n 's/^host: //p'`

# Default seconds per target for `just fuzz`.

fuzz_smoke_secs := "20"

# Each tool recipe builds in a target directory of its own, so the gate's
# streams can overlap (see `gate`). Guest wasm paths are passed explicitly
# to producer and consumers alike, with `--target-dir`, so an ambient
# CARGO_TARGET_DIR cannot separate a guest from the harness that reads it.

fuzzfit_target := justfile_directory() + "/target/fuzzfit"
fuzzfit_guest_wasm := fuzzfit_target + "/wasm32-unknown-unknown/fuel/fuzzfit_guest.wasm"

fuelscape_target := justfile_directory() + "/target/before-fuelscape"
surfacecheck_target := justfile_directory() + "/target/surfacecheck"

wasm32pins_target := justfile_directory() + "/target/wasm32-pins"
wasm32pins_guest_wasm := wasm32pins_target + "/wasm32-unknown-unknown/wasm32-pins/wasm32_pins_guest.wasm"

board_fuel_target := justfile_directory() + "/target/board-fuel"
board_fuel_guest_wasm := board_fuel_target + "/wasm32-unknown-unknown/fuel/board_fuel_guest.wasm"

focused_fuel_target := justfile_directory() + "/target/focused-fuel"
focused_fuel_guest_wasm := focused_fuel_target + "/wasm32-unknown-unknown/fuel/focused_fuel_guest.wasm"

# The environment for every fuel-metered guest build. Embedded source paths
# (panic locations) shift a guest's memory layout and so its fuel, so this
# remaps the host-specific prefixes (CARGO_HOME and the toolchain's rust-src)
# and clears any rustc wrapper: every host builds the same module. It is
# shell text, prefixed to the cargo command.

fuel_guest_env := "RUSTC_WRAPPER= RUSTFLAGS=\"--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo --remap-path-prefix=$(rustc --print sysroot)/lib/rustlib/src/rust=/rustc/$(rustc -vV | sed -n 's/^commit-hash: //p')\""

suanpan_fuel_target := justfile_directory() + "/target/suanpan-fuel"
suanpan_fuel_guest_wasm := suanpan_fuel_target + "/wasm32-unknown-unknown/fuel/suanpan_fuel_guest.wasm"

# List recipes.
default:
    @"{{ just_executable() }}" --justfile "{{ justfile() }}" --list --unsorted

# ── inner loop ───────────────────────────────────────────────────────────────

# Type-check every host target; targets with required-features wait for the gate.
check:
    cargo check --all-targets

# Run the test suites; pass a filter to narrow (`just test mirror`).
test *args:
    cargo nextest run {{ args }}

# Run the test suites under every feature (the gate's test run).
test-all *args:
    cargo nextest run --all-features --no-fail-fast {{ args }}

# Package-scoped: before's operation meters pin the dev profile.

# Run rumors' tests at CI's property-case count, in the release profile.
test-release *args:
    PROPTEST_CASES={{ proptest_ci_cases }} cargo nextest run --all-features --locked --cargo-profile release --profile high-count --no-fail-fast {{ args }}

# Nightly's merged mode compiles one doctest harness per crate rather than
# one binary per example, whose link time dominated the gate. A separate
# target dir keeps the two compilers from invalidating each other.

# Run the doctests (nightly), which nextest does not run.
doctest:
    RUSTDOCFLAGS="-Z unstable-options --merge-doctests yes" cargo +{{ nightly_toolchain }} test --doc --all-features --target-dir target/doctest-nightly

# Lint every target, warnings denied (the commit-gate setting).
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# `clippy` runs under --all-features, and the dev-dependency cycle turns on
# the meter and oracle features in every test build, so surface dead under
# default features never trips it. These lines lint the default-feature
# builds, one package at a time so feature unification cannot turn the
# features back on. The bare `--lib` line lints the shipped library alone:
# building test targets alongside it keeps cfg(test) modules alive.

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
# Each tools/ checker runs its self-test first, so a checker bug fails the
# gate instead of passing everything. Each tool's header documents its rules.

# doclint and testdoc walk named roots, never `.`, whose untracked trees
# (agent worktrees under `.claude/`) would otherwise enter the verdict.

# Flag doc-comment summaries and included-figure layouts that render wrong.
doclint:
    ./tools/doclint --self-test
    ./tools/doclint benches crates examples src tests

# Require every Rust test to document the behavior and invariant it protects.
testdoc:
    ./tools/testdoc --self-test
    ./tools/testdoc benches crates examples src tests

# Hold every workflow step to committed or immutably-pinned code.
workflowlint:
    ./tools/workflowlint --self-test
    ./tools/workflowlint .github

# Cargo checks neither rule: `cargo metadata` resolves inheritance away, and
# cargo cannot tell a tool from any other member.

# Hold member manifests to the workspace dependency table, and tools to the default-member split.
manifestlint:
    ./tools/manifestlint --self-test
    ./tools/manifestlint

# Fails when the committed wire captures stop matching the meter's patterns,
# never on the measured digest share.

# Check the wire-capture renderer vocabulary via the digest-share meter.
digestshare:
    ./tools/digestshare --self-test
    ./tools/digestshare

# The READMEs derive from crate-level rustdoc via cargo-rdme, with the
# intra-doc links it cannot resolve stripped to code spans; never hand-edit
# them. Needs cargo-rdme at the version tools/readme pins.

# Regenerate every crate's README from its crate-level rustdoc.
readme:
    ./tools/readme write

# Verify every README is in sync with its rustdoc (the gate variant of `readme`).
readme-check:
    ./tools/readme self-test
    ./tools/readme check

# The rustdoc is the documentation of record, so broken intra-doc links fail
# the gate. A page missing the fuelscape widget's assets shows empty charts
# and fails nothing else, so tools/fuelscape-assets requires them exactly
# once on every page with a chart and on no other.

# Build the rustdoc with warnings denied; hold the widget assets to their pages.
docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
    ./tools/fuelscape-assets --self-test
    ./tools/fuelscape-assets target/doc

# Rendering private items catches stale links in private modules, but
# silences `private_intra_doc_links`, so neither pass replaces `docs`. Its
# own target dir keeps the two from re-documenting each other's output.

# Build the rustdoc including private items, warnings denied; hold the widget assets to their pages.
docs-internal:
    RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --document-private-items --target-dir target/doc-internal
    ./tools/fuelscape-assets --self-test
    ./tools/fuelscape-assets target/doc-internal/doc

# The only leg that builds rumors' `doc_cfg` path and each crate's
# `[package.metadata.docs.rs]`. cargo-docs-rs takes the features and rustdoc
# arguments from that metadata and appends RUSTDOCFLAGS to them, so the
# warnings are denied under exactly the configuration docs.rs renders. It
# cannot check the published tarball: `cargo package` on before refuses
# until its `suanpan` dependency carries a version. Needs
# `cargo install cargo-docs-rs`.

# Build the rustdoc as docs.rs does (pinned nightly, `--cfg docsrs`, each crate's docs.rs metadata), warnings denied.
docs-docsrs:
    RUSTDOCFLAGS="-D warnings" cargo +{{ nightly_toolchain }} docs-rs -p rumors --target-dir target/doc-docsrs
    RUSTDOCFLAGS="-D warnings" cargo +{{ nightly_toolchain }} docs-rs -p before --target-dir target/doc-docsrs
    RUSTDOCFLAGS="-D warnings" cargo +{{ nightly_toolchain }} docs-rs -p suanpan --target-dir target/doc-docsrs
    ./tools/fuelscape-assets target/doc-docsrs/{{ host_triple }}/doc

# A `cfg` that compiled tests/future_size.rs empty would read as a pass.

# Fail unless the future-size pins were collected and pass (liveness for tests/future_size.rs).
future-size:
    cargo nextest run --all-features -E 'binary_id(rumors::future_size)' --no-tests=fail

# Run the amplification board's smoke suite, which a plain nextest run skips.
amp-board-smoke:
    cargo nextest run --all-features --ignore-default-filter -E 'binary_id(before::amp_board_smoke)' --no-tests=fail

# cargo-audit checks every tracked Cargo.lock against RustSec, the gate's
# one network fetch: vulnerabilities fail, other advisories warn. cargo-deny
# holds the default members to one version per crate; deny.toml names each
# tolerated duplicate's holdout, and tool-only duplicates are exempt because
# they never ship. Needs `cargo install cargo-audit cargo-deny`.

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
    # cargo-deny never reads default-members, so the tool-side members are
    # excluded by name.
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

# The fuzz targets live in a detached workspace no root command reaches.
# Its tests replay every committed seed; the coverage-guided run is
# `just fuzz`, too slow and nondeterministic for the gate.
# Needs `cargo install cargo-fuzz`.

# Lint and test the detached fuzz crate, and compile every libFuzzer target (nightly).
[working-directory("crates/before/fuzz")]
fuzz-build:
    cargo fmt --check
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked --lib
    cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

# The build-free lints run first, so a formatting slip costs seconds; then
# every building leg runs concurrently. Concurrency cannot move a verdict,
# because every verdict (counters, fuel, virtual time) is unaffected by load.
#
# Legs sharing a target directory serialize on its build lock, so every leg
# building in the root `target/` shares one stream, cheap-first. A cargo
# `build.build-dir` puts every build behind one lock, which costs wall time
# but changes no verdict. Nothing caps peak memory, which concurrency
# multiplies.

# Run the pre-commit gate; it must come up fully clean before every commit.
gate: gate-lints gate-streams

# Run the build-free gate tier, sequentially.
gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare readme-check

# Each stream logs to its own file, and failing streams' logs replay in full
# at the end. A stream that records neither an ok nor a failed marker
# (killed from outside, say by the OOM killer) fails the gate.

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
        # The marker is written before the narration that reports it.
        # Re-invoking the running just on this justfile honors `-f`, `-d`,
        # or another `just` first on PATH; variable overrides given on the
        # gate's command line do not reach the streams.
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

    # The workspace stream carries the test suite and is the critical path,
    # so the other streams run niced, filling the cores it leaves idle.
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
    # A stream with no marker died without a verdict: it fails the gate, and
    # its partial log replays too.
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

# The gate's build roster, one group per stream; CI reuses these groups.
# `_gate-wasm` holds both legs that build the fuzz-fit guest.
_gate-workspace: clippy clippy-default docs test-all future-size amp-board-smoke
_gate-doctest: doctest

_gate-board: amp-board-acceptance

_gate-board-fuel: amp-board-fuel

_gate-suanpan-fuel: suanpan-fuel

_gate-focused-fuel: focused-fuel

_gate-wasm: fuzzfit fuelscape-test wasm32-pins
_gate-fuzz: fuzz-build
_gate-surface: surface-totality
_gate-internal-docs: docs-internal
_gate-docsrs: docs-docsrs
_gate-audit: supply-chain

# ── artifacts the gate doesn't reach ─────────────────────────────────────────
# `serde,borsh` is checked as a pair because both derive on the same types.

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

# Exactly what the Pages deploy runs. Needs wasm-pack and npm (network on
# first run).

# Full visualizer build: wasm-pack, strict TypeScript typecheck, esbuild bundle.
viz:
    ./crates/before-viz/build.sh

# Nothing else compiles the benches.

# Compile (don't run) the criterion benches.
bench-build:
    cargo bench --no-run

# Each run reads the committed `seeds/<target>/` and writes discoveries to
# the untracked `corpus/<target>/`, so the seeds stay pristine. Targets come
# from `cargo fuzz list`, so a new one cannot miss the sweep.

# Short fuzz smoke: run each libFuzzer target for `secs` seconds.
[working-directory("crates/before/fuzz")]
fuzz secs=fuzz_smoke_secs:
    #!/usr/bin/env bash
    set -euo pipefail
    while IFS= read -r target; do
        # libFuzzer requires its write corpus to exist.
        mkdir -p "corpus/$target"
        cargo +{{ nightly_toolchain }} fuzz run --target {{ host_triple }} "$target" "corpus/$target" "seeds/$target" -- -max_total_time={{ secs }}
    done < <(cargo +{{ nightly_toolchain }} fuzz list)

# The fuzz-fit harness replays generated programs natively and under
# Wasmtime fuel, judging each step against its band in harness/src/bands.rs;
# a cost change fails until re-pinned with `just fuzzfit-calibrate`.
#
# Tool-side members like these are built only by recipes that name them, so
# every tool recipe that runs tests also runs the members' clippy.

# Build the fuzz-fit wasm guest and its harness (both halves).
fuzzfit-build:
    {{ fuel_guest_env }} cargo build -p fuzzfit-guest --profile fuel --target wasm32-unknown-unknown --target-dir {{ fuzzfit_target }}
    cargo build -p fuzzfit-harness --tests --release --target-dir {{ fuzzfit_target }}

# Run the fuzz-fit asymptotics suites against the pinned fuel bands.
fuzzfit: fuzzfit-build
    cargo clippy -p fuzzfit-guest -p fuzzfit-harness --all-targets --target-dir {{ fuzzfit_target }} -- -D warnings
    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo nextest run --ignore-default-filter -p fuzzfit-guest -p fuzzfit-harness --cargo-profile release --target-dir {{ fuzzfit_target }}

# Byte-reproducible from the committed corpus, so any diff is a real change:
# review it like a snapshot and annotate the movement in the commit message.

# Re-fit and rewrite the fuzz-fit harness's pinned fuel bands.
fuzzfit-calibrate: fuzzfit-build
    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo run --release -p fuzzfit-harness --bin calibrate --target-dir {{ fuzzfit_target }}

# The pins run `before` at the 32-bit boundaries on inputs synthesized
# inside the guest, which neither `wasm-check` (compile only) nor a 64-bit
# host can exercise.

# Build the 32-bit boundary-pin wasm guest and its harness (both halves).
wasm32-pins-build:
    cargo build -p wasm32-pins-guest --profile wasm32-pins --target wasm32-unknown-unknown --target-dir {{ wasm32pins_target }}
    cargo build -p wasm32-pins-harness --tests --release --target-dir {{ wasm32pins_target }}

# Run the 32-bit boundary pins under wasmtime (minutes; a few GiB of host memory).
wasm32-pins: wasm32-pins-build
    cargo clippy -p wasm32-pins-guest -p wasm32-pins-harness -p wasm32-pins-protocol --all-targets --target-dir {{ wasm32pins_target }} -- -D warnings
    WASM32_PINS_GUEST_WASM={{ wasm32pins_guest_wasm }} cargo nextest run --ignore-default-filter -p wasm32-pins-guest -p wasm32-pins-harness -p wasm32-pins-protocol --cargo-profile release --target-dir {{ wasm32pins_target }}

# Each fuel reading is judged against another reading of the same program,
# so nothing is calibrated and no toolchain bump needs re-pinning; the
# harness's crate docs state the bounds. Both rustdoc builds run because
# each catches broken links the other cannot.

# Build the interleaving-fuel wasm guest and its harness (both halves).
suanpan-fuel-build:
    {{ fuel_guest_env }} cargo build -p suanpan-fuel-guest --profile fuel --target wasm32-unknown-unknown --target-dir {{ suanpan_fuel_target }}
    cargo build -p suanpan-fuel-harness -p suanpan-fuel-protocol --tests --release --target-dir {{ suanpan_fuel_target }}

# Run suanpan's interleaving-fuel property, its skeleton census, and its focused cost claims.
suanpan-fuel: suanpan-fuel-build
    cargo clippy -p suanpan-fuel-guest --target wasm32-unknown-unknown --target-dir {{ suanpan_fuel_target }} -- -D warnings
    cargo clippy -p suanpan-fuel-harness -p suanpan-fuel-protocol --all-targets --target-dir {{ suanpan_fuel_target }} -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc -p suanpan-fuel-harness -p suanpan-fuel-protocol --no-deps --target-dir {{ suanpan_fuel_target }}/doc-public
    RUSTDOCFLAGS="-D warnings" cargo doc -p suanpan-fuel-harness -p suanpan-fuel-protocol --no-deps --document-private-items --target-dir {{ suanpan_fuel_target }}/doc-internal
    RUSTDOCFLAGS="-D warnings" cargo doc -p suanpan-fuel-guest --target wasm32-unknown-unknown --no-deps --target-dir {{ suanpan_fuel_target }}/doc-public
    RUSTDOCFLAGS="-D warnings" cargo doc -p suanpan-fuel-guest --target wasm32-unknown-unknown --no-deps --document-private-items --target-dir {{ suanpan_fuel_target }}/doc-internal
    SUANPAN_FUEL_GUEST_WASM={{ suanpan_fuel_guest_wasm }} cargo nextest run --ignore-default-filter -p suanpan-fuel-harness -p suanpan-fuel-protocol --cargo-profile release --target-dir {{ suanpan_fuel_target }}

# The pipeline smoke measures fuel in the fuzz-fit guest, hence the
# dependency. Audit only: the board and the fuzz-fit bands enforce fuel.

# Lint and test the fuelscape: sampler adequacy pins plus the pipeline smoke.
fuelscape-test: fuzzfit-build
    cargo clippy -p before-fuelscape --all-targets --target-dir {{ fuelscape_target }} -- -D warnings
    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo nextest run --ignore-default-filter -p before-fuelscape --target-dir {{ fuelscape_target }}

# One log-log fuel-by-size heatmap per public operation, with the committed
# adversarial families overlaid; deterministic per seed and plan. Options
# pass through, e.g. `just fuelscape --samples 500 --max-bytes 512`.

# Render the full population atlas into target/fuelscape (audit view; not enforcement).
[working-directory("crates/before-fuelscape")]
fuelscape *args: fuzzfit-build
    FUELSCAPE_TIP=$(git rev-parse HEAD) FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} cargo run --release -p before-fuelscape --bin fuelscape --target-dir {{ fuelscape_target }} -- --out {{ justfile_directory() }}/target/fuelscape {{ args }}

# The widget datasets in crates/before/fuelscape derive from the committed
# gzipped dump, and `fuelscape-verify` (CI only; it takes about a minute)
# checks they match. To re-measure: `just fuelscape --dump --samples <n>
# --max-bytes <n>`, `gzip -9 -n` the dump into crates/before-fuelscape/dump,
# run `just fuelscape-compact`, and commit the dump, datasets, and any claim
# changes together.

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

# The README links the figure by URL, so it is committed; before's build.rs
# holds it fresh, and the variable opts the build into writing it.

# Regenerate the README's space-consumption figure from the measurement artifact.
doc-figure:
    BEFORE_REGEN_DOC_FIGURE=1 cargo build -p before

# The widget bundle and the claim strings meet only in the browser, so
# nothing compiled checks them. CI only, keeping the gate node-free.

# Check the widget bundle's syntax and every committed claim's grammar.
fuelscape-claims:
    node --check crates/before/docs/fuelscape.js
    ./tools/fuelscape-claims

# ── the formal tier (formal/lean; needs elan) ────────────────────────────────
# Local only: the CI runner has no Lean toolchain.

# Kernel-check the Lean theorem artifact (all proofs, pins, controls).
[working-directory("formal/lean")]
lean:
    PATH="$HOME/.elan/bin:$PATH" lake build

# Fewer seeds than the default fail by design: the vacuity controls need
# enough runs to reproduce the known adversarial stalls.

# Run the event-DAG oracle and schedule gate; the seed count only goes up.
[working-directory("formal/lean")]
eventdag fuzz_seeds="100":
    PATH="$HOME/.elan/bin:$PATH" lake build eventdag
    PATH="$HOME/.elan/bin:$PATH" lake exe eventdag eventdag-out {{ fuzz_seeds }}

# The golden file is formal/lean/muxprobe-expected.tsv. After a deliberate
# model or matrix change, regenerate it inside formal/lean with
# `lake exe muxprobe --update` and review the diff like a snapshot.

# Run the mux executable-evidence matrix against its committed golden file.
[working-directory("formal/lean")]
muxprobe rand_seeds="25":
    PATH="$HOME/.elan/bin:$PATH" lake build muxprobe
    PATH="$HOME/.elan/bin:$PATH" lake exe muxprobe {{ rand_seeds }}

# ── conveniences ─────────────────────────────────────────────────────────────

# The temp file keeps a failed build from truncating the table; the window
# suite fails if the committed table drifts from this rendering.

# Regenerate the sync-budget trade-off table compiled into the rustdoc.
window-tradeoff:
    cargo run --example window_tradeoff > src/tree/mirror/streaming/window/tradeoff.md.tmp
    mv src/tree/mirror/streaming/window/tradeoff.md.tmp src/tree/mirror/streaming/window/tradeoff.md

# Full sampling is required for any quoted number. The filter matches
# Criterion IDs (`group/function`), and further arguments pass through, e.g.
# `just bench version version/hole --profile-time 10` to profile.

# Run one bench target at full sampling, through a Criterion filter.
bench target *filter:
    cargo bench --bench {{ target }} -- {{ filter }}

# The reduced-sampling inner loop: `bench` at 10 samples x 1 s. Never quoted.
bench-quick target *filter:
    cargo bench --bench {{ target }} -- --sample-size 10 --measurement-time 1 {{ filter }}

# The amplification board judges each cell's peak heap, a deterministic
# counter, against its declared bounds, so its output is byte-identical
# under any load and any shard count (AMP_BOARD_SHARDS). It runs in release,
# the profile of record: debug assertions add work, so never pin a dev
# reading. A bare `just amp-board [scale]` is a debugging view; the verdict
# of record is `amp-board-acceptance`.

# No root-target leg builds release, so a separate target dir costs no extra
# compilation and never waits on the workspace stream's build lock.
amp_board_target := justfile_directory() + "/target/amp-board"

# One command for every board recipe, so the profile and target dir are part
# of the instrument.
amp_board_command := "cargo run --release -p before --example amp_board --features serde,borsh --target-dir " + amp_board_target

# Prints the git tree of the tracked files as they stand, edits included, so
# a capture names the source it measured and calibration can refuse another
# tree's readings. It stages into a temporary index copy, never taking
# `.git/index.lock` or touching the real index.
source_tree := 'd=$(mktemp -d) && trap "rm -rf \"$d\"" EXIT && cp "$(git rev-parse --git-path index)" "$d/index" && GIT_INDEX_FILE="$d/index" git add -u && GIT_INDEX_FILE="$d/index" git write-tree'

# Render the board at one scale (a debugging view), or `compare` two captures.
[positional-arguments]
amp-board *args:
    {{ amp_board_command }} -- "$@"

# One sweep of each cell's whole ladder, judged twice, each judgment run
# whatever the other's verdict:
#
# - Acceptance judges every constant and floor per size and fits each
#   exponent as one trend. A failing cell is resolved only by a fix or an
#   owner-declared model at the cell.
# - The ranking pin requires the live worst family per operation x currency
#   to match WORST_RANKINGS, whose docs give the TARGET_DEPENDENT rule. A
#   flip is news: re-pin a legitimate overtake with a movement annotation,
#   and investigate one a code change caused.
#
# `--capture <file>` also writes the exact readings, stamped with the source
# tree, for `just amp-board compare` and calibration.

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

# Runs every board cell in a wasm32 guest under Wasmtime fuel, a count of
# every executed instruction, judged by the board's rules against per-cell
# calibrated ceilings; the native board stays the measure of record for
# heap. Each cell gets a fresh guest, so readings ignore load and
# AMP_BOARD_FUEL_THREADS. The ceilings record the compiler, Wasmtime, and
# cost table they were calibrated under, and the run refuses a guest built
# under any other: after a bump, run `just amp-board-calibrate-fuel`.
# `--capture <file>` works as for the native board.

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

# Each cell's ceilings derive from its own readings (the board's
# `cell_ceilings` module states the rule). The heap table must hold on both
# architectures of record (`board::HEAP_ARCHS`), so re-deriving it takes two
# hosts at one source tree:
#
# 1. On one architecture, `just amp-board-acceptance --capture <file>`.
# 2. On the other, `just amp-board-calibrate <file>` with an absolute path.
#    It sweeps this host, refuses a capture of another tree or architecture
#    set, takes each sample's larger reading, rewrites the heap table, and
#    then runs `amp-board-calibrate-fuel`.
#
# Fuel does not depend on the host, so a fuel-only change needs one host:
# `just amp-board-calibrate-fuel <file>` from an `amp-board-fuel --capture`,
# or with no capture after a toolchain, Wasmtime, or cost-table change (the
# fuel board then refuses to capture).
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

# Each claim relates readings of one guest, so no fuel number is committed
# and nothing needs recalibrating; the harness's crate docs name the forms.

# Build the focused-fuel wasm guest and its harness's tests.
focused-fuel-build:
    {{ fuel_guest_env }} cargo build -p focused-fuel-guest --profile fuel --target wasm32-unknown-unknown --target-dir {{ focused_fuel_target }}
    cargo build -p focused-fuel-harness -p focused-fuel-protocol --tests --release --target-dir {{ focused_fuel_target }}

# Run the cost claims the board cannot state, each judged in Wasmtime fuel.
focused-fuel *args: focused-fuel-build
    cargo clippy -p focused-fuel-guest --target wasm32-unknown-unknown --target-dir {{ focused_fuel_target }} -- -D warnings
    cargo clippy -p focused-fuel-harness -p focused-fuel-protocol --all-targets --target-dir {{ focused_fuel_target }} -- -D warnings
    FOCUSED_FUEL_GUEST_WASM={{ focused_fuel_guest_wasm }} cargo nextest run --ignore-default-filter -p focused-fuel-harness -p focused-fuel-protocol --cargo-profile release --target-dir {{ focused_fuel_target }} {{ args }}

# Every public function in `before`, per nightly rustdoc JSON, needs a board
# disposition or a named exception; trait impls and non-function items are
# reconciled against the censuses in crates/before/surfacecheck/src/census.rs.
# The checker refuses JSON of any `format_version` but its pin's;
# `nightly_toolchain` gives the bump procedure.

# Build the nightly rustdoc JSON the surface-totality check parses.
surface-json:
    cargo +{{ nightly_toolchain }} rustdoc -p before --lib --all-features --target-dir target/surface-json -- -Z unstable-options --output-format json

# Hold before's public surface total against resource coverage.
surface-totality: surface-json
    cargo clippy -p surfacecheck --all-targets --target-dir {{ surfacecheck_target }} -- -D warnings
    cargo nextest run --ignore-default-filter -p surfacecheck --target-dir {{ surfacecheck_target }}
    cargo run -q -p surfacecheck --target-dir {{ surfacecheck_target }} -- {{ justfile_directory() }}/target/surface-json/doc/before.json

# The maximum over the committed roster, not a proof of the true worst case.
# amp-board-acceptance checks the rankings; this view adds runner-ups and
# margins.

# Render the worst-case map: the argmax family per operation x currency, one table per sampling scale.
worst-cases:
    {{ amp_board_command }} -- worst-cases

# ── the no-rot sweep ─────────────────────────────────────────────────────────
# Each recipe below owns one CI job, and the workflow calls it by name rather
# than restating its members. The wasm and fuel instruments run only in the
# gate, not on shared runners.

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
