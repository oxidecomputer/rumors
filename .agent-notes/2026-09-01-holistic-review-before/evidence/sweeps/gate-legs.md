# Sweep gate-legs: Gate legs, CI, and derived-artifact freshness for before and suanpan

## Method and coverage

Verification pass over the gate-legs sweep at 9e5784fb (working tree clean,
confirmed by `git rev-parse HEAD` and `git status`). For every finding I
opened the cited sites with line numbers (`cat -n`, `awk`), grepped use
sites, and checked git history for a recorded rationale.

Read in full: the justfile (1042 lines), .github/workflows/ci.yml,
.cargo/mutants.toml, tools/mutantcheck-expected.json, rust-toolchain.toml,
deny.toml, crates/before/fuzz/Cargo.toml and README.md, both detached
`.cargo/config.toml` files, tools/benchjudge-expected.json, and
crates/before/AGENTS.md. Read in ranges: tools/mutantcheck (1-160),
tools/testdoc (1-80), tools/doclint (360-400), tools/covcheck (1-100),
tools/benchjudge (55-95), tools/citecheck (285-335), crates/before/src/surface.rs
(55-160 plus every row carrying `pins: &[]`), crates/before/src/version/rank.rs
(396-428), crates/before/src/testing/surface_coverage/tests.rs (100-150,
190-240), crates/before/tests/meter.rs (1-45, 6850-6865, 6895-6940,
7430-7475), crates/before/tests/bench_judge_roster.rs (40-62),
crates/before/src/lib.rs (398-414), the fuzzfit harness's bands.rs (60-100,
200-210, 315-325), bin/calibrate.rs (60-70, 340-410), tests/enforce.rs
(14-40, 439), and crates/before/surfacecheck/Cargo.toml (15-32).

Mechanical checks run: `just --list --unsorted` (captured to the scratch
dir); `find . -name Cargo.lock` (six lockfiles in the tree, plus a copy under
another agent's `.claude/worktrees/` checkout, left alone); `gh run view` on
runs 33567211421 (HEAD, coverage red), 33560347645 (previous green main run,
head 3327a92b), and 33429688343 (2026-08-31 instruments red), with the ci and
coverage job logs saved and grepped; `git diff --stat 3327a92b 9e5784fb --
crates/before crates/suanpan` (empty) and the Cargo.lock package diff between
those two commits (blake3/arrayref/arrayvec/constant_time_eq out; sha3,
keccak, digest, crypto-common, hybrid-array, const-oid, sponge-cursor in; no
reverse dependency of any of these is in before's or suanpan's closure, per
crates/before/Cargo.toml, crates/suanpan/Cargo.toml, and a Cargo.lock
dependency walk); `sort -u target/mutants-raw.txt | wc -l` (54,918 distinct
mutants); two Python scans over `proptest!` blocks in the in-scope crates
(245 fn heads; 244 carry `#[test]` in source, the one exception a nested
helper); `cargo mutants --version` (27.1.0 locally).

One permitted test run: `cargo nextest run -p before -p suanpan
--all-features -E 'test(masked_cmp_hole_envelope)'` with
`--success-output immediate` added so the MEASURED line is shown for a
passing test; exit 0, `peak_heap=384`. The second permitted run was not
needed.

Not re-run here (reported by the sweep, unverified by me): `just
readme-check`, `just fuelscape-verify`, `just citecheck`, `just testdoc`,
`just manifestlint`, `just fuelscape-claims`. Not runnable here: any
coverage leg, so the mechanism behind gate-legs-1 stays open.

## Findings

### gate-legs-1: The coverage leg is red at HEAD on a byte-identical before tree; the heap meter is not deterministic under instrumentation
- Where: crates/before/tests/meter.rs:6860-6860 (related: crates/before/tests/meter.rs:13-21, crates/before/tests/meter.rs:6907-6910, crates/before/tests/meter.rs:6929-6933, justfile:1031-1035, .github/workflows/ci.yml:226-227)
- Class / severity / confidence: correctness / high / high
- Provenance: verified (`gh run view 33567211421` and `--log-failed`; `gh run view 33560347645` with its coverage job log; `git diff --stat 3327a92b 9e5784fb -- crates/before crates/suanpan` empty; Cargo.lock package diff walked against before's and suanpan's dependency closures); executed: yes: `cargo nextest run -p before -p suanpan --all-features -E 'test(masked_cmp_hole_envelope)' --success-output immediate`, exit 0, `peak_heap=384`
- Verification: confirmed, and sharpened: the last green coverage run (3327a92b) and the red one (9e5784fb) have byte-identical crates/before and crates/suanpan trees, and the Cargo.lock movement between them touches only rumors' hash crates, none of which any before or suanpan dependency reaches; history: no-rationale-found
- Owner-gated: no

GitHub run 33567211421 fails the `coverage` job at `just coverage-kernel` with `masked_cmp_hole_envelope` reading peak heap 1156 B against the 480 B pin, while the previous main run's coverage job passed on an identical before tree and test-binary closure, and the same test reads 384 B uninstrumented (the pin is exactly 384 x 1.25). The peak-heap column the suite documents as one of three deterministic meters is therefore not a function of the tree under the coverage leg, and the gate never runs that leg, so no local check tells a committer that main is red.

Evidence:

        13	//! Three deterministic meters, asserted together per scenario:
      6860	    pub const MASKED_CMP_HOLE: QueryEnvelope = query_envelope(480, 0, 0, 7_535, 18, 0, 10); // the block skip consumes the spine's unowned continuation whole: [...]
      6907	    HEAP.reset_peak_usage();
      6908	    let baseline = HEAP.current_usage();
      6909	    let r = f();
      6910	    let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
      1034	    {{ justfile_directory() }}/tools/memwatch cargo llvm-cov nextest --workspace --all-features --lcov --output-path target/llvm-cov/workspace.lcov

    CI job 100053091628 (coverage, run 33567211421):
        MEASURED masked_cmp_hole: input_bytes=755 peak_heap=1156 segments=0 limb_ops=0 touches=14 scan_bits=6028
        masked_cmp_hole: peak heap 1156 B exceeds the pinned envelope 480 B (input 755 B): note: the meters are process-global and meaningful only one scenario per process: run under cargo nextest, not a shared-process cargo test

    Local, uninstrumented (this review):
        MEASURED masked_cmp_hole: input_bytes=755 peak_heap=384 segments=0 limb_ops=0 touches=14 scan_bits=6028

Resolution: Settle the source before touching the pin. Run `just coverage-kernel` twice at 9e5784fb and compare the `MEASURED masked_cmp_hole` lines with the uninstrumented 384. If the instrumented reading moves between identical runs, find what allocates inside the scenario body only under `-C instrument-coverage` (or only sometimes) and either isolate the meter from it or exclude the heap column under coverage builds with the reason stated at the exclusion. If the instrumented reading is stable but differs from 384, the envelope suite and the coverage leg judge different profiles, and the coverage recipes should filter the meter suite out (they exist to measure kernel coverage, not envelopes). Never widen 480 to accommodate. Acceptance: two consecutive green `coverage` jobs on main with no change to any envelope constant, and a comment at the chosen site naming the mechanism.
Construction: The comparison above already demonstrates the claim: identical trees, one green and one red coverage run, and a local reading of 384 B. The residual question is which of the two remedies applies, which only the coverage leg itself can answer.

### gate-legs-2: cargo-mutants is version-pinned by the count roster but installed unpinned in CI, and the install comment misclassifies it
- Where: .github/workflows/ci.yml:76-86 (related: tools/mutantcheck-expected.json:2, tools/mutantcheck:145-151, justfile:308-311, justfile:326)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (read ci.yml:76-86, mutantcheck:137-151, mutantcheck-expected.json:2; `cargo mutants --version` = 27.1.0 locally; `git log -S'"tool":'` shows the pin landing in bec1ceaf on 2026-08-13 and the CI install landing in e4d92ae4 on 2026-08-17 without it); executed: no
- Verification: confirmed; history: no-rationale-found (e4d92ae4's message says only that the runner must carry the binary; the ci.yml comment's rationale for not pinning is false for this tool)
- Owner-gated: no

tools/mutantcheck refuses any `cargo mutants --version` other than the string pinned in tools/mutantcheck-expected.json, so `just ci`'s `mutants-list` leg turns red on an untouched tree the day taiki-e/install-action's manifest resolves the next cargo-mutants release. The step comment justifies pinning only cargo-rdme on the grounds that every other tool merely reports findings; cargo-mutants' version string and operator set are committed expectations compared exactly.

Evidence:

        76	      # cargo-rdme carries a version because it is the only tool here whose
        77	      # output is a committed artifact compared byte for byte: readme-check
        80	      # in the repo, turning the sweep red on a tree nobody touched. The others
        81	      # report findings rather than generate bytes, so they ride the pinned
        82	      # action's own tool manifest, moving when Dependabot bumps the pin.
        86	          tool: just,cargo-nextest,cargo-rdme@2.1.0,cargo-fuzz,cargo-mutants,wasm-pack

    tools/mutantcheck-expected.json
         2	  "tool": "cargo-mutants 27.1.0",

    tools/mutantcheck
       146	    if tool_version != pinned_tool:
       147	        problems.append(
       148	            f"tool version {tool_version!r} does not match the pinned "
       149	            f"{pinned_tool!r}: operator sets move between releases, so "
       150	            "re-pin the counts in the same diff as the tool bump"

Resolution: Pin `cargo-mutants@27.1.0` beside `cargo-rdme@2.1.0` on ci.yml:86 and reword lines 76-82: the pinned tools are those whose version is a committed expectation (cargo-rdme's emitted bytes; cargo-mutants' version string and operator inventory), and a bump touches the expectation file and the install line in one diff. Acceptance: the workflow names the same cargo-mutants version as tools/mutantcheck-expected.json, and tools/workflowlint (or a one-line grep in `mutants-list`) fails when the two disagree.
Construction: Install any other cargo-mutants release and run `just mutants-list`: the checker exits 1 with `tool version ... does not match the pinned 'cargo-mutants 27.1.0'` before comparing a single count.

### gate-legs-3: wasm32-pins is missing from every hand-enumerated roster of detached workspaces; its lockfile is never audited and its local-only status is undocumented
- Where: justfile:328-352 (related: deny.toml:4, rust-toolchain.toml:3-4, justfile:14-18, justfile:389-390, justfile:979-986, .github/workflows/ci.yml:110-120, crates/before/wasm32-pins/Cargo.lock)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`find . -name Cargo.lock` lists six lockfiles under the tree; `grep -c '^name = "wasmtime' crates/before/wasm32-pins/Cargo.lock` = 10; read every cited range; `git log -S'wasm32-pins' -- justfile` shows eb6ba627 as the only touch and its message wires the leg into the gate's wasm stream only; `gh run view 33429688343 --log-failed` shows the 2026-08-31 instruments red was cargo-audit on `lru` and `wasmtime`); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The supply-chain leg says it sweeps every lockfile in the repository but audits five; crates/before/wasm32-pins/Cargo.lock, which carries wasmtime (the crate whose advisory turned the instruments job red on 2026-08-31), is never audited. The same four-name enumeration recurs in deny.toml, rust-toolchain.toml, the gate-streams collision argument, and the CI instruments job's "what stays local" roster, none of which names wasm32-pins; the leg runs only in the gate's wasm stream and in none of `ci`, `instruments`, or `all`.

Evidence:

       329	# cargo-audit sweeps every lockfile in the repository — the root workspace
       330	# and each detached workspace (fuzz, fuzzfit, fuelscape, surfacecheck) —
       347	    cargo audit
       348	    cargo audit --file crates/before/fuzz/Cargo.lock
       349	    cargo audit --file crates/before/fuzzfit/Cargo.lock
       350	    cargo audit --file crates/before-fuelscape/Cargo.lock
       351	    cargo audit --file crates/before/surfacecheck/Cargo.lock

    deny.toml
         4	# detached workspaces (fuzz, fuzzfit, fuelscape, surfacecheck) are

    rust-toolchain.toml
         3	# directory, so the fuzz, fuzzfit, fuelscape, and surfacecheck
         4	# workspaces all take it too.

    .github/workflows/ci.yml
       117	  #   - the wasmtime fuel tier (the fuzzfit bands and the fuelscape pins):
       118	  #     deterministic, but it brings a wasm32 guest build plus wasmtime to
       119	  #     police asymptotics the board legs below already judge at the scales
       120	  #     of record — the gate keeps the second jaw.

Resolution: Add `cargo audit --file crates/before/wasm32-pins/Cargo.lock` to `supply-chain`, or drive the list from `find . -name Cargo.lock -not -path '*/target/*'` inside the recipe so the enumeration cannot rot again; fix the four prose rosters; then either add `wasm32-pins` to the `instruments` job or add it to that job's "what stays local, and why" comment with the reason (justfile:634-636 says it peaks at a few GiB across nextest workers). Acceptance: `just supply-chain` names every lockfile `find` reports, and every roster that enumerates detached workspaces names wasm32-pins or is replaced by a mechanical enumeration.
Construction: Add a crate with a current RustSec advisory to crates/before/wasm32-pins/Cargo.toml, regenerate its lockfile, and run `just supply-chain`: exit 0.

### gate-legs-4: manifestlint is a gate lint that CI never runs
- Where: justfile:1000-1000 (related: justfile:403, justfile:198-209, .github/workflows/ci.yml:107-108)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read justfile:403 and :1000; `git show 84044759 -- justfile` shows `gate-lints` gaining `manifestlint` while the `ci` line is untouched; `grep -rn manifestlint .github/` empty); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`gate-lints` includes manifestlint but the `ci` recipe, which the GitHub `ci` job runs verbatim, omits it, and neither of the other jobs runs it; a member manifest restating a version lands green in CI whenever the committer skips the gate. The omission dates from the commit that added the tool.

Evidence:

       403	gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check
      1000	ci: fmt-check doclint testdoc workflowlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz

    git show 84044759 -- justfile
    -gate-lints: fmt-check doclint testdoc workflowlint digestshare mutants-list readme-check
    +gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check

Resolution: Add `manifestlint` to the `ci` line (build-free, seconds), and consider defining `ci` as `gate-lints` plus its build legs so the two rosters cannot diverge again. Acceptance: every recipe named in `gate-lints` appears in `ci`, ideally by construction.

### gate-legs-5: ci.yml installs and describes floating toolchains the recipes never invoke; the dated toolchains arrive by rustup auto-install
- Where: .github/workflows/ci.yml:128-133 (related: .github/workflows/ci.yml:17-20, .github/workflows/ci.yml:54-74, .github/workflows/ci.yml:139-153, .github/workflows/ci.yml:202-216, justfile:23-40, rust-toolchain.toml:20-23)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read ci.yml in full and justfile:23-40; the `ci` job log of run 33567211421 shows `info: syncing channel updates for nightly-2026-06-30-x86_64-unknown-linux-gnu` at the first `cargo +nightly-2026-06-30` invocation and `the toolchain '1.97.1-x86_64-unknown-linux-gnu' is currently in use (overridden by ... rust-toolchain.toml)` during both dtolnay steps; the coverage job log of run 33560347645 shows cargo-llvm-cov running `rustup component add llvm-tools-preview` for both `1.97.1` and `nightly-2026-06-30`; `git show --stat e7a4b7b0` touched justfile, rust-toolchain.toml, AGENTS.md, bands.rs but not ci.yml); executed: no
- Verification: confirmed, with the mechanism now attested from CI logs rather than inferred; history: deliberate-but-expired (the floating installs predate e7a4b7b0's pinning commit, which did not update the workflow)
- Owner-gated: no

Every nightly recipe invokes `cargo +nightly-2026-06-30` and rust-toolchain.toml pins stable at 1.97.1, yet all three jobs install floating `nightly` and `stable` (with `llvm-tools` on both in the coverage job), the prose says the recipes invoke `cargo +nightly` and that the instruments job "tracks nightly, so a format bump upstream can turn the leg red", which the dated pin exists to make impossible. The jobs work because rustup auto-installs the dated nightly at first use and cargo-llvm-cov self-installs `llvm-tools-preview` on the toolchains actually used; the workflow states neither.

Evidence:

        56	      # The doctest and fuzz recipes invoke `cargo +nightly`, so nightly only
        57	      # needs to exist, not be the default — whereas the gate's bare
        66	        with:
        67	          toolchain: nightly
       130	  # format_version. This job tracks nightly, so a format bump upstream can
       131	  # turn the leg red on an untouched tree — the checker refuses loudly,
       206	      - name: Install nightly toolchain (branch coverage)
       209	          toolchain: nightly
       210	          components: llvm-tools

    justfile
        40	nightly_toolchain := "nightly-2026-06-30"

    ci job log, run 33567211421 (ANSI stripped, truncated):
        Install nightly toolchain ... info: note that the toolchain '1.97.1-x86_64-unknown-linux-gnu' is currently in use (overridden by '/home
        just ci ... cargo +nightly-2026-06-30 test --workspace --doc ...
        just ci ... info: syncing channel updates for nightly-2026-06-30-x86_64-unknown-linux-gnu

    coverage job log, run 33560347645 (truncated):
        info: running `rustup component add llvm-tools-preview --toolchain 1.97.1-x86_64-unknown-linux-gnu` to install the `llvm-tools-preview` component for the selected to
        info: running `rustup component add llvm-tools-preview --toolchain nightly-2026-06-30-x86_64-unknown-linux-gnu` to install the `llvm-tools-preview` component for th

Resolution: Install what the recipes name: `toolchain: nightly-2026-06-30` in each nightly step (with `components: llvm-tools` in the coverage job), sourced from one place so the pin cannot fork (a workflow `env` the justfile variable is checked against, or a step that reads `nightly_toolchain` from the justfile); drop the floating `stable` steps and let rust-toolchain.toml provision stable (add `llvm-tools` to its `components` if the coverage job should not rely on cargo-llvm-cov's self-install); rewrite lines 17-20, 54-58, and 128-133 for the pinned regime and its paired bump procedure. Acceptance: CI logs show no `syncing channel updates` for a toolchain the workflow did not name, and the workflow prose names the same nightly date as justfile:40.

### gate-legs-6: No recipe runs the mutation campaign, so the roster's kill claims are never re-attested
- Where: .cargo/mutants.toml:26-29 (related: .cargo/mutants.toml:48-51, justfile:294-326, tools/mutantcheck:21-23, tools/mutantcheck:54-55)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read .cargo/mutants.toml, tools/mutantcheck, justfile:294-326 and 1000-1003; `just --list` shows no campaign recipe; `sort -u target/mutants-raw.txt | wc -l` = 54,918; `.agent-notes/2026-08-21-cbor-wire-mutants-scopeA/README.md` records a scoped campaign of 3141 mutants at 16 jobs); executed: no
- Verification: confirmed, resolution reframed for scale; history: no-rationale-found (the roster's 2026-08-18 commits describe campaigns run by hand; nothing records a cadence or the last run)
- Owner-gated: no

The roster header names `cargo mutants --workspace` as the campaign of record and lists mutants that named instruments kill, but the only cargo-mutants invocations in the tree are the two `--list` runs; the checker's own docstring leaves "a full campaign" as the authority on any mutant's fate while nothing runs one. A test weakened so that one of the "deliberately absent" mutants survives produces no signal in `gate`, `ci`, or `all`. At 54,918 distinct listed mutants, a full workspace campaign is not a per-commit leg, which is exactly why its cadence needs to be decided and recorded rather than left implicit.

Evidence:

        26	# Campaign configuration of record — the keys below make the plain
        27	# invocation (`cargo mutants --workspace`) run it: nextest, all
        28	# features, the whole workspace's suites, under cargo's default
        48	# Deliberately absent: sweep::eq_exit's `||`-guard mutant and
        49	# integral::meter_product's tap deletion. Their owning instruments kill
        50	# them — the eq_exit early-exit row and the settle-products liveness
        51	# floor in tests/meter.rs — so they need no exclusion. Likewise absent:

    justfile
       324	    cargo mutants --list --colors=never --no-config --workspace > target/mutants-raw.txt
       325	    cargo mutants --list --colors=never --workspace > target/mutants-filtered.txt

    tools/mutantcheck
        21	silently swallows every NEW mutant the function later grows — both rot
        22	invisibly because no gate leg runs cargo-mutants at all. This checker
        54	and a full campaign remains the authority on any individual mutant's
        55	fate.

Resolution: Add a campaign recipe of record (`cargo mutants --workspace` under the roster's keys) and a cheaper `--in-diff` variant for PRs; give the full campaign a cadence that fits its cost (a scheduled, sharded CI job, or a committed dated attestation of the last full run that a lint leg holds fresh), and record the expected outcome (zero MISSED under the roster) so a survivor is a red, not a report. Acceptance: a recipe exists whose exit status is the campaign verdict, and something in the tree or CI names when it last ran.
Construction: Delete the `eq_exit` early-exit row assertion in tests/meter.rs and run `just gate && just ci && just all`: all green; only `cargo mutants --workspace` (or a `--in-diff` campaign over that change) reports the survivor.

### gate-legs-7: `all` is documented as "Everything" while omitting the gate's instrument legs and the coverage legs, and its exclusive legs have no recorded cadence
- Where: justfile:1002-1003 (related: justfile:7, justfile:14-18, crates/before/tests/bench_judge_roster.rs:54-56, tools/benchjudge-expected.json:2)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read justfile:1-19 and 975-1003, ci.yml in full, bench_judge_roster.rs:45-62; `gh run list --workflow ci` shows only the `ci` workflow on main); executed: no
- Verification: reframed: the sweep rated this medium with a claim that the crate docs' "fuzzed codecs" sentence is unbacked; the fuzz targets exist, build in the gate, and their seeds are gated, so that sentence describes real verification and is not a false claim. What survives is the misnomer and the absence of any record of when the `all`-only legs last ran; the cadence decision is an open question for the owner rather than a defect; history: deliberate-and-holds for the manual tier (justfile:991-997 explains why each leg is local), no-rationale-found for the word "Everything"
- Owner-gated: no

`all`'s doc line reads "Everything" although the header's own next paragraph says neither sweep repeats the gate's instrument legs, and `all` also omits both coverage legs. The fuzz smoke, the formal tier, and the bench judge run only in `all`, which no workflow invokes and nothing records; the bench roster's one required red is re-attested only by habit.

Evidence:

         7	#   no-rot sweep just ci / just all                  everything, so nothing rots
        14	# what CI cannot run (the fuzz smoke and the formal tier). Neither sweep
        15	# repeats the gate's instrument legs — the fuel bands, the board verdicts
        16	# and pins, and surface totality run in `just gate`, and GitHub CI's
      1002	# Everything: the no-rot sweep, plus the fuzz smoke, the formal tier, and the bench judge.
      1003	all: ci (fuzz fuzz_smoke_secs) lean eventdag muxprobe bench-judge bench-judge-tripwire

    crates/before/tests/bench_judge_roster.rs
        54	/// means the tripwire went dark. Rostered reds are re-attested at
        55	/// `just all` cadence: the bench-judge recipes run there with the machine
        56	/// to themselves, while the gate's parallel tier judges no wall time.

Resolution: Rename the doc line and header entry to what `all` is (the no-rot sweep plus the manual tier), or make `all` include `gate` so the word is true. For cadence, see the open question below: either a scheduled workflow for the shared-runner-safe legs (the fuzz smoke) or a committed attestation of the last `all` run. Acceptance: `just --list` describes `all` accurately, and the tree or CI states when the `all`-only legs last ran.

### gate-legs-8: Five writer-sink rows in the surface roster are excluded on all three legs with empty pins; their claimed doctest pin is unenforceable
- Where: crates/before/src/surface.rs:506-511 (related: crates/before/src/surface.rs:745-750, crates/before/src/surface.rs:785-790, crates/before/src/surface.rs:797-802, crates/before/src/surface.rs:987-992, crates/before/src/surface.rs:74-85, crates/before/src/surface.rs:239-245, crates/before/src/version/rank.rs:412-420, crates/before/src/testing/surface_coverage/tests.rs:212-217)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of every `pins: &[]` row in surface.rs; read the five rows and their buffer-door neighbors; grep for `encode_to(` in every tests.rs under crates/before/src shows one named test, `encode_to_matches_encode` at codec/tests.rs:812, covering Party, Version, and Clock only, cited at surface.rs:245; the five doors' byte-identity assertions live only in doctests at version.rs:1088, rank.rs:418, ranked.rs:182, ranked.rs:231, span/wire.rs:68; the coverage suite extends its resolvable set only from `pins`); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`Version::encode_rank_to`, `Rank::encode_to`, `Ranked::encode_to`, `Ranked::encode_rank_to`, and `Span::encode_to` carry `NoWireFormatInReferences { pins: &[] }` on prod_tree, prod_fs, and tree_fs alike, so the row cites nothing checkable. The family's documentation says each writer-sink door is pinned by its doctest, but doctests are outside both the nextest inventory citecheck resolves against and the coverage suite's payload check; deleting those doctests leaves every gate leg green. The Party/Version/Clock writer doors show the intended shape: `codec_row` carries CODEC_PINS on prod_tree and `encode_to_matches_encode` is cited by name.

Evidence:

       506	    SurfaceRow {
       507	        op: "Version::encode_rank_to",
       508	        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
       509	        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
       510	        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
       511	    },
        80	    /// format goldens, and, for each writer-sink door, its doctest
        81	    /// pinning byte identity with the buffer door.
       245	            pins: &["encode_to_matches_encode"],

    crates/before/src/testing/surface_coverage/tests.rs
       214	                Exclusion::NoWireFormatInReferences { pins }
       215	                | Exclusion::DefinitionalCombinator { pins }
       216	                | Exclusion::NAryNotInReferences { pins }
       217	                | Exclusion::LinearityMechanics { pins } => names.extend(*pins),

    crates/before/src/version/rank.rs
       418	    /// rank.encode_to(&mut buf).unwrap();
       419	    /// assert_eq!(buf, rank.encode());

Resolution: Extend `encode_to_matches_encode` (or add one law over every `*_to` door) to cover the rank, ranked, and span writers, cite it in each of the five rows' `pins`, and add a coverage-suite assertion that a row excluded on all three legs cites at least one resolvable name, so the vacuous shape cannot recur. Acceptance: no roster row has three excluded legs with an empty union of pins, and the new assertion fails when one is introduced.
Construction: Delete the `# Example` block at rank.rs:412-420 and run `just gate`: green (no leg names `Rank::encode_to`).

### gate-legs-9: Two rosters carry a class for accepting known failures (one now empty, one holding only a demonstration)
- Where: tools/covcheck:11-15 (related: tools/covcheck:77, justfile:1006-1009, tools/benchjudge:65-69, tools/benchjudge-expected.json:12-14, crates/before/tests/bench_judge_roster.rs:57-59)
- Class / severity / confidence: scaffolding / low / medium
- Provenance: verified (read tools/covcheck:1-100; disposition tally over tools/covcheck-expected.json: 26 panic-arm, 8 unreachable, 0 remediation; read benchjudge:55-95 and benchjudge-expected.json; `git log -S'remediation' -- tools/covcheck-expected.json` shows remediation entries existed and were drained in aa7c96a0 and f77011e3 on 2026-08-12); executed: no
- Verification: confirmed; history: deliberate-but-expired (the class carried real entries for one day and has been empty since; the justfile's GOAL statement still names it)
- Owner-gated: yes: the class is a design choice stated at justfile:1006-1009, and the doctrine it conflicts with is the owner's

covcheck admits a "remediation" disposition for a reachable-but-unexercised kernel line, and benchjudge describes its `red` class as where "owned reds await their cures". No covcheck entry uses "remediation" today and the only rostered red is the schoolbook tripwire, a required red that demonstrates the judge is alive rather than an accepted failure, so both are empty buffers for known failures, which the doctrine says may not exist even empty.

Evidence:

        14	invariant it rests on), and "remediation" (reachable but unexercised: a
        15	named open population gap, never a blessed exception).
        77	    dispositions = {"panic-arm", "unreachable", "remediation"}

    justfile
      1008	# panic-arm or an unreachable arm, its argument stated at the entry) or a
      1009	# named remediation item, and any NEW hole fails by name. MECHANISM:

    tools/benchjudge
        67	makes the judge enforce a *fixed* verdict
        68	map instead of all-green, so `just all` stays meaningful while owned reds
        69	await their cures. The file pins the configuration it was recorded under

    tools/benchjudge-expected.json
        12	  "red": [
        13	    "display_schoolbook/hugeleaf"
        14	  ]

Resolution: Drop "remediation" from covcheck's disposition set and from the justfile's GOAL sentence (a reachable uncovered kernel line then blocks until a directed test lands or it is curated as panic-arm or unreachable). Redefine benchjudge's `red` class as the required-red known-bad tripwires and strike the "owned reds await their cures" sentence, so an unexpected red has exactly two exits: a cure, or a sidecar-declared model. Acceptance: covcheck's self-test pins that "remediation" is refused, and benchjudge's docstring and the roster notes describe `red` only as the liveness tripwire set.

### gate-legs-10: doclint sweeps in-tree build outputs; only two of five detached workspaces redirect their target dirs to dodge it
- Where: tools/doclint:370-373 (related: tools/testdoc:19, crates/before/fuzzfit/.cargo/config.toml:1-7, crates/before-fuelscape/.cargo/config.toml:1-7, justfile:181, justfile:365-368, justfile:629-631, justfile:940-945)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read doclint:370-373 and testdoc:19; `find crates/before/fuzz/target -name '*.rs'` lists two generated thiserror `private.rs` files; `crates/before/surfacecheck/target` and `crates/before/wasm32-pins/target` exist in-tree with no `.rs` today; `ls` shows `.cargo/config.toml` only under fuzzfit and before-fuelscape); executed: no
- Verification: confirmed; history: deliberate-and-holds for the two redirects (their comments state the doclint reason), no-rationale-found for the asymmetry
- Owner-gated: no

doclint's walk has no `target` exclusion (testdoc's does) and the gate runs it over `crates`, so generated sources under the fuzz workspace's in-tree target are linted as if committed, and surfacecheck and wasm32-pins build in-tree too. fuzzfit and fuelscape carry a `.cargo/config.toml` whose only stated purpose is to route around this; the other three detached workspaces do not, so a dependency bump that emits a long doc summary into an in-tree OUT_DIR turns doclint red on code the tree does not own.

Evidence:

       370	def rust_files(root):
       371	    if root.is_file():
       372	        return [root] if root.suffix == ".rs" else []
       373	    return sorted(root.rglob("*.rs"))

    tools/testdoc
        19	IGNORED_DIRECTORIES = {".git", "node_modules", "target"}

    crates/before/fuzzfit/.cargo/config.toml
         1	# Build into the repo root's target/ (its own subdirectory) instead of a
         2	# workspace-local target dir: the gate's doclint sweeps every .rs under
         3	# crates/, and wasmtime's build scripts generate rustdoc'd sources that
         4	# would otherwise land inside this tree and fail it. The repo root target/
         5	# is outside every gate sweep.

    find crates/before/fuzz/target -name '*.rs'
    crates/before/fuzz/target/aarch64-apple-darwin/release/build/thiserror-c36e32fcf4d5360f/out/private.rs
    crates/before/fuzz/target/debug/build/thiserror-c60931e4e8dce25d/out/private.rs

Resolution: Exclude `target`, `.git`, and `node_modules` in doclint's walk exactly as testdoc does, with a self-test fixture for the skip; then the two `.cargo/config.toml` redirects lose their stated reason and can be dissolved or re-justified at the file. Acceptance: doclint's self-test pins that a `.rs` under a `target/` directory is not visited, and no `.cargo/config.toml` cites doclint as its reason.

### gate-legs-11: The fuzz workspace's prose carries opaque roster tags, floating-nightly instructions, a ghost test name, and a hand-duplicated duration
- Where: crates/before/fuzz/Cargo.toml:1-9 (related: crates/before/fuzz/Cargo.toml:48, crates/before/fuzz/README.md:1, crates/before/fuzz/README.md:13-16, crates/before/fuzz/README.md:25, crates/before/fuzz/README.md:54-60, crates/before/src/clock/tests.rs:772-773, justfile:51-54)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both files in full; grep for `(COV|PROG)-[0-9]+` across the in-scope crates, tools, justfile, and .cargo finds only these three sites; grep for `h34` across crates/before finds only README.md:25, while the live test is `decode_never_panics` at clock/tests.rs:773); executed: no
- Verification: confirmed, with one ghost reference added; history: no-rationale-found
- Owner-gated: no

The fuzz manifest and README carry "PROG-5 / COV-7" roster tags, instruct `cargo +nightly fuzz build` and `rustup toolchain install nightly` while the recipes of record use the dated `nightly_toolchain` and `--target {{ host_triple }}`, and the README names a test `clock::tests::h34_decode_never_panics` that does not exist (the live test is `clock::tests::decode_never_panics`), which breaks the root AGENTS.md hard rule against references to code that no longer exists. The justfile also restates the 20 s smoke duration by hand as matching the manifest's guidance.

Evidence:

         1	# Standalone fuzz workspace (PROG-5 / COV-7). The empty `[workspace]` table detaches
         4	#   cargo +nightly fuzz build
        48	# keystone invariant (COV-7) inline — an accepted value re-encodes stably and decodes

    crates/before/fuzz/README.md
         1	# `before` fuzz targets (PROG-5 / COV-7)
        14	rustup toolchain install nightly
        25	  the in-tree proptest `clock::tests::h34_decode_never_panics`.
        55	cargo +nightly fuzz build   # build all targets

    crates/before/src/clock/tests.rs
       772	    #[test]
       773	    fn decode_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..512)) {

    justfile
        51	# Default fuzz smoke duration per target, in seconds (matches the guidance in
        52	# crates/before/fuzz/Cargo.toml).

Resolution: Delete the tags; fix the test name to `clock::tests::decode_never_panics`; replace the manual command lists with `just fuzz-build` / `just fuzz` (keeping the seed-corpus explanation); drop the duration duplication from the justfile comment or make the manifest defer to the recipe. Acceptance: no `PROG-`/`COV-` tag remains in the in-scope tree, every test name in the fuzz README resolves against `cargo nextest list -p before --all-features`, and the fuzz prose names the dated toolchain or the recipe rather than `+nightly`.

### gate-legs-12: `just --list` renders eight recipe descriptions as sentence fragments
- Where: justfile:2-3 (related: justfile:111-114, justfile:287-289, justfile:314-321, justfile:363-366, justfile:588-594, justfile:633-641, justfile:705-710, justfile:792-807)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (ran `just --list --unsorted`, saved under the scratch dir, and read the eight comment blocks); executed: yes: `just --list --unsorted`
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The header calls `just --list` the tour, but just uses the comment line immediately above a recipe as its description, so recipes whose explanatory block abuts the recipe line show that block's last line: test-all, citecheck, mutants-list, fuzz-build, fuzzfit, wasm32-pins, doc-figure, and bench-alloc-ab. The file's own convention (a blank line, then a one-sentence doc comment, as at lines 120-123) is broken at these eight sites.

Evidence:

         2	# the workspace has a recipe here, tiered by feedback speed, and `just --list`
         3	# is the tour.

    just --list --unsorted
        test-all *args                              # module build only here).
        citecheck                                   # collected test inventory.
        mutants-list                                # capture, and this flag is what keeps that refusal from ever firing.
        fuzz-build                                  # workspace's formatting leg: the root `cargo fmt --all` cannot reach it.
        fuzzfit                                     # surfacecheck recipes carry the same discipline).
        wasm32-pins                                 # gates — the fuzzfit recipes carry the same discipline).
        doc-figure                                  # that lets the build write into the source tree.
        bench-alloc-ab target arm="shipped" *filter # (never quoted).

Resolution: Insert a blank line and a one-sentence doc comment above each of the eight recipes. Acceptance: every line of `just --list` reads as a complete sentence.

### gate-legs-13: Hand-maintained counts and dated measurements in verification prose
- Where: justfile:580-583 (related: justfile:116, justfile:33, justfile:342, crates/before/fuzzfit/harness/src/bands.rs:74-77, crates/before/fuzzfit/harness/src/bands.rs:98, crates/before/fuzzfit/harness/src/bin/calibrate.rs:347-353, crates/before/fuzzfit/harness/tests/enforce.rs:439, tools/benchjudge-expected.json:2, crates/before/surfacecheck/Cargo.toml:22-24)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read every cited range; `grep -rn 'BANDS.len()'` over the fuzzfit harness finds no census test; a rustdoc fence tally over crates/before/src counts 380 fence lines, about 190 code blocks, of which 2 are `text` and 3 `compile_fail`); executed: no
- Verification: confirmed, with one more stale count found ("nearly 100" doctests against roughly 185 compiled blocks); history: no-rationale-found
- Owner-gated: no

Several verification comments restate enumerable facts by hand: the fuzz-fit sentry sizes (48 and 256), "nearly 100" doctest examples, "measured: 84 crates vs 459", the rustdoc format number duplicated between the justfile and surfacecheck's manifest; bands.rs's module doc states "49 band keys: 44 kernels" twice in the head that `calibrate` preserves verbatim while regenerating everything below its splice marker, and no test compares the count to `BANDS.len()`, so a re-pin that changes the key census leaves both sentences stale; benchjudge-expected.json's notes carry measured exponents as provenance prose.

Evidence:

       116	# Stable rustdoc compiles one executable per example; `before` has nearly 100,
       342	# silently check the root package alone (measured: 84 crates vs 459).
       582	# (48 fuzzed programs against the pinned bands, point and shape legs,
       583	# plus the whole 256-program deterministic prefix judged step by step:

    crates/before/fuzzfit/harness/src/bands.rs
        76	//! the toolchain in [`PINNED_RUSTC`], wasmtime 47 fuel. 49 band keys: 44
        77	//! kernels, of which five have sampled rejection arms (`clock_join` and
        98	//! All 49 band keys read linear-or-flatter within every family above the

    crates/before/fuzzfit/harness/src/bin/calibrate.rs
       347	    let marker = "/// The toolchain that pinned";
       351	    let head = &current[..current
       352	        .find(marker)
       353	        .expect("bands.rs splice marker present")];

    crates/before/fuzzfit/harness/tests/enforce.rs
       439	    #![proptest_config(ProptestConfig::with_cases(48))]

    tools/benchjudge-expected.json:2 (excerpt)
        (the declared riders measured e 0.93–1.18 at their pinning)

Resolution: Name the constants instead of their values in the justfile (`ProptestConfig::with_cases` in enforce.rs, `REFIT_PREFIX_PROGRAMS`, the calibrate default); have `calibrate` emit the band-key census into the generated region or drop it from the head, or add a test holding the head's count to `BANDS.len()`; replace "84 vs 459" and "nearly 100" with the mechanism statements alone; move the measured exponents out of the roster notes into the commit that pinned them. Acceptance: no number in these comments can be changed by a code edit that does not touch the comment, or a test fails when one drifts.

## Positives

- gate-streams carries a liveness floor on its own verdict (justfile:409-413, 478-481): a stream killed without writing an ok or failed marker fails the gate and its partial log is replayed, so an OOM-killed stream can never read as a pass. Read and confirmed.
- Every tools/ checker leads with a `--self-test` that pins its red paths against synthetic fixtures, refuses a missing root as a usage error rather than a clean sweep (doclint:387-394 read; testdoc and workflowlint per the sweep), and carries liveness floors so an extractor that stops matching fails by name (mutantcheck:81-84, 129-133 read).
- Provenance is bound to runs, not memory: mutantcheck pins the tool version and never writes its expectation file (mutantcheck:75-79 read); benchjudge refuses a roster inventing a ceiling class (benchjudge:85-88 read); tools/readme refuses any cargo-rdme but the pinned 2.1.0 and CI installs exactly that (ci.yml:76-86 read).
- The coverage pin is tamper-evident in both directions and anchored on source text with an offset, failing closed on a vanished or ambiguous anchor (covcheck:16-27, 70-100 read), and the design reason for rejecting a global threshold is stated at the recipe (justfile:1014-1015).
- The mutants exclusion roster's three line-pinned entries (watermark.rs:790:43, grow.rs:463:38, prescan.rs:337:26) each state genre, the leg suppressed per replacement operator, and the reason the sibling legs face the suite (mutants.toml:99-117, 139-142, 146-150 read); the header's disposition ladder (refactor, assert, exclude last) is stated as standing policy.
- The `proptest!` convention is uniform: every property test writes `#[test]` in source (244 of 244), which is exactly what lets testdoc's lexical checker and citecheck's inventory see them.
- CI invokes justfile recipes rather than re-listing their steps (ci.yml:14-15), so the only rosters that can drift are the recipe lines themselves, which is where gate-legs-4 found the one divergence.
- The wasm32-pins workspace keeps overflow checks on at release so a 32-bit wrap traps rather than wraps (justfile:624-625; eb6ba627's message records the pins landing red-first with adjacency witnesses).

## Open questions for Finch

1. What allocates the extra heap in `masked_cmp_hole_envelope` under `cargo llvm-cov nextest` (1156 B) but not under `cargo nextest run` (384 B), and why did the previous coverage run on an identical before tree pass? I could not run the coverage leg. The answer decides gate-legs-1's remedy: isolate the meter, or drop the envelope suite from the coverage recipes.
2. Is wasm32-pins' absence from the CI `instruments` job deliberate (runner memory, per justfile:634-636) or an oversight from eb6ba627? Either answer belongs in that job's "what stays local, and why" comment (gate-legs-3).
3. What cadence do you want for the legs that run only at `just all` (the fuzz smoke, the formal tier, the bench judge) and for a mutation campaign if gate-legs-6 lands: a scheduled workflow for the shared-runner-safe legs, a committed attestation of the last local run that a lint leg checks for staleness, or an explicit statement at each recipe that it is manual? At 54,918 listed mutants the full campaign cannot be a per-commit leg.
4. The nightly bump procedure (justfile:35-38) names only surface-totality, but the same nightly drives doctest, fuzz-build, and coverage-kernel-branch; the stable bump procedure (rust-toolchain.toml:16-18) names only fuzzfit-calibrate while covcheck's pins are toolchain-sensitive (justfile:1019-1022). Should both procedures enumerate every leg the pin feeds?
5. The codec reviewer should answer whether the inline insta goldens in crates/before/src/testing/snapshots.rs pin the byte layout of every codec door, since a self-consistent wrong wire format passes round-trips, laws, both byte-blind oracles, and the fuzz-seed gate; gate-legs-8 is the writer-sink corner of the same question.

## Dropped

- Sweep finding [9] (testdoc cannot see tests declared inside `proptest!` blocks): false. The `proptest!` macro passes `$(#[$meta])*` through and adds no `#[test]` of its own (proptest-1.11.0 src/sugar.rs:155-159), and every property test in the in-scope crates writes `#[test]` inside the block (244 of 245 fn heads; the one without is the nested helper `depth_by_recursion` at query/tests.rs:1351, not a test), so testdoc's TEST_ATTRIBUTE regex matches each one and `has_attached_doc` walks up from that attribute to the `///`. The checker's contract holds for property tests as written; the sweep's construction (an undocumented property test passing testdoc) cannot be built without also dropping `#[test]`, which stops the fn being a test at all.
