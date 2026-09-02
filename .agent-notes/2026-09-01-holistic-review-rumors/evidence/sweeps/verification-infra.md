# Sweep verification-infra: Verification infrastructure audit

## Method and coverage

This is the verification pass over the sweep's eighteen findings, at commit
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean). For each
finding I opened the cited sites with line numbers, checked the claim
against the code, and looked for a recorded rationale in git history and
`.agent-notes/`.

Read in full, with line numbers: `justfile` (1042 lines),
`.github/workflows/ci.yml` (230), `tests/future_size.rs` (91),
`tools/testdoc` (151), `tools/digestshare` (108), `.config/nextest.toml`,
`rust-toolchain.toml`, `tests/gossip_pipelining.rs` (94). Read in part:
`tools/mutantcheck` (header, lines 1-90, and 130-145), `tools/covcheck`
(195-225), `.cargo/mutants.toml` (1-60), `tools/covcheck-expected.json`
(1-12), `tools/mutantcheck-expected.json` (1-20), `design/rumors-frame-fuzz.md`
(1-30, 265-335), `src/lib.rs` (20-60), `tests/tradeoff_probe.rs` (1-35,
180-195), `tests/window_knee.rs` (1-40 plus a grep of its test roster),
`AGENTS.md` (140-175), `tests/dispute_wire.rs` (1-40), `Cargo.toml`
(94-110, 170-200), `results/mirror-complexity.tex` (1-12, 34-42, 96-106),
`tests/routed_link.rs`, `tests/changes.rs`, `tests/handshake.rs`,
`src/link/routed/tests.rs`, `src/link/routed/header/tests.rs` (110-135),
`.agent-notes/2026-08-21-cbor-wire-mutants-scopeA/README.md` (1-60 and a
grep), `.agent-notes/2026-08-21-unknown-pruning-survivor/README.md` (1-30),
one snapshot (`tests/snapshots/bootstrap_snapshot__empty_provider.snap`,
1-30).

Mechanical checks I ran (read-only): `grep` for every `nextest run` line in
the justfile and every `--release`/`--cargo-profile release` line in the
justfile's and ci.yml's whole history; `grep -c proptest` over every file
in `tests/` and over `src/conformance/**`; a Python scan of every
`proptest! {` block under `src`, `tests`, `examples`, `benches` counting
depth-1 `fn` items and whether each carries `#[test]` and a `///` line
(118 found, 0 missing either); `grep -l '│' tests/snapshots/*.snap` (0 of
22) and a grep for hexdump-shaped lines (0); `find .claude -name '*.rs' |
wc -l` (543); `git log -S`/`-L` on the `ci:` and `gate-lints:` recipe
lines, `manifestlint`, `bench = false`, `nightly_toolchain`, the
install-action tool list, `tools/digestshare`, `tests/future_size.rs`,
`design/rumors-frame-fuzz.md`, and `results/`; `gh run list`/`gh run view`
on the CI workflow for the last eight main runs and the failing job's log
at HEAD; a scratch file under my own scratchpad directory run through
`python3 tools/testdoc` to demonstrate the `proptest!` blind spot.

Not done, and therefore not claimed: I ran no `cargo`, `just`, build, or
test command (the two permitted `cargo nextest run` invocations were not
needed; every correctness-adjacent claim here settles by reading). I did
not reproduce `just --list`, so the listing fragments in
verification-infra-13 are assessed from the file's comment layout and
just's documented rule, with the sweep's reported output as unverified
corroboration. I did not run `tools/testdoc` on the foreign worktree under
`.claude/worktrees/`. I did not build `tests/future_size.rs` under the
release profile, so whether its budget holds today is unknown (open
question). The `before` meter failure that reds CI's coverage job is out
of this partition; I recorded what the logs show and stopped.

## Findings

### verification-infra-1: The public-future size guardrail is compiled out of every wired test run
- Where: tests/future_size.rs:17-20 (related: Cargo.toml:180-182, justfile:108-114, justfile:597, justfile:644, justfile:1031-1042, .github/workflows/ci.yml:107-108, .github/workflows/ci.yml:226-230)
- Class / severity / confidence: verification-gap / high / high
- Provenance: verified (read the cfg, the profile comment stating debug assertions stay on in dev, and every `nextest run` line in the justfile and its history; ci.yml runs `just ci` and the two coverage legs, all dev-profile)
- Verification: confirmed; history: no-rationale-found (the cfg has been on the file since bdf10d4c4; no root-workspace release-profile test run has ever existed in the justfile or ci.yml history — the only `--cargo-profile release` lines, past and present, belong to the detached fuzzfit and wasm32-pins workspaces)
- Owner-gated: no

The whole test binary is gated `#![cfg(not(debug_assertions))]`, the dev
profile keeps debug assertions on by design, and every root-workspace test
recipe (`test`, `test-all`, the two `coverage-kernel` legs) and every CI job
runs the dev profile. The three budget tests therefore compile to an empty
binary in every wired run: nextest reports zero tests for it and nothing
notices. The regression it names (the protocol's `Levels` chain leaking
into a public future and tripping downstream `recursion_limit`) would pass
every check.

Evidence:

    tests/future_size.rs
        17	//! The budget is enforced only in release builds: debug layouts carry
        18	//! additional state, and they are not what users ship.
        19	
        20	#![cfg(not(debug_assertions))]

    Cargo.toml
       180	# `debug-assertions` stays on, as the dev profile grants: the envelope
       181	# suite's limb, scan, and touch pins count work the `debug_assert!`
       182	# comparisons perform, and those are what `--release` would remove.

    justfile
       113	test-all *args:
       114	    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace --all-features {{ args }}
       597	    FUZZFIT_GUEST_WASM={{ fuzzfit_guest_wasm }} {{ justfile_directory() }}/tools/memwatch cargo nextest run --cargo-profile release
       644	    WASM32_PINS_GUEST_WASM={{ wasm32pins_guest_wasm }} cargo nextest run --cargo-profile release

    .github/workflows/ci.yml
       107	      - name: just ci
       108	        run: just ci

Resolution: add a release-profile run of this one binary to the gate's
workspace stream (for example `cargo nextest run -p rumors --cargo-profile
release -E 'binary(future_size)'` under `tools/memwatch`), or make the
budget hold under the dev profile with a dev-specific constant so the
existing legs exercise it; in either case add a liveness guard that the
three tests were collected (nextest's `--no-tests=fail`, or a count check
in the recipe), since the failure mode here is exactly an empty binary
reading as a pass. Acceptance: the gate log shows the three
`future_size` tests running, and deleting the `Box::pin` erasure (a
scratch edit, reverted) turns the leg red.
Construction: run the binary under `--cargo-profile release` once first;
its budget has never been exercised in any wired run, so whether it
passes today is itself unknown (see open questions).

### verification-infra-2: The coverage legs sit in no composite recipe, so the full local ladder is green while CI's coverage job is red at this commit
- Where: justfile:1002-1003 (related: justfile:12-14, justfile:1005, justfile:1017-1018, .github/workflows/ci.yml:186-230, justfile:1031-1042)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the `ci`/`all` recipe lines and the coverage section; `gh run list --workflow ci --branch main` and `gh run view 33567211421 --json jobs` / `--log-failed`, read-only)
- Verification: reframed — the sweep said "no local recipe runs the CI coverage legs"; the recipes `coverage-kernel` and `coverage-kernel-branch` exist and a developer can run them by name. What is true is narrower: neither `gate`, `ci`, nor `all` depends on them, so no composite invocation reaches them, and the header's "Everything" for `all` overclaims. Severity lowered from high to medium accordingly; the red itself is a `before` failure outside this partition. History: deliberate-and-holds for the gate exclusion (justfile:1005, 1017-1018 state it is too slow for the gate); no-rationale-found for their absence from `all`.
- Owner-gated: no

GitHub CI runs three jobs (ci, instruments, coverage). At HEAD the run for
this push (33567211421) is red: `coverage` failed at `just coverage-kernel`
while `ci` and `instruments` passed. The failure is
`before::meter masked_cmp_hole_envelope` (a peak-heap envelope, 1156 B
measured against a 480 B pin, under llvm-cov instrumentation; the same
test passes uninstrumented in the `ci` job at the same commit). Across the
last six main runs the coverage job was red twice (9e5784fb, 00e83cf7) and
green three times, with only `Cargo.lock` differing under `crates/before`
between the last green (3327a92b) and this red. Whatever the mechanism, a
developer running `just all` had no way to see it.

Evidence:

    justfile
         2	# the workspace has a recipe here, tiered by feedback speed, and `just --list`
         3	# is the tour.
        12	# `ci` builds the artifacts the gate doesn't reach (the feature matrix, wasm,
        13	# bench builds, the viz bundle), exactly as GitHub CI builds them; `all` adds
        14	# what CI cannot run (the fuzz smoke and the formal tier). Neither sweep
      1002	# Everything: the no-rot sweep, plus the fuzz smoke, the formal tier, and the bench judge.
      1003	all: ci (fuzz fuzz_smoke_secs) lean eventdag muxprobe bench-judge bench-judge-tripwire
      1005	# ── the coverage legs (CI cadence; the gate never runs them) ─────────────────

    gh run view 33567211421 --json jobs
        {"conclusion":"failure","name":"coverage"}
        {"conclusion":"success","name":"instruments"}
        {"conclusion":"success","name":"ci"}

    gh run view 33567211421 --log-failed (ANSI stripped by hand)
        FAIL [   0.009s] ( 741/1812) before::meter masked_cmp_hole_envelope
        masked_cmp_hole: peak heap 1156 B exceeds the pinned envelope 480 B (input 755 B)
        error: recipe `coverage-kernel` failed on line 1034 with exit code 100

Resolution: add `coverage-kernel coverage-kernel-branch` to `all` (they
are deterministic-verdict legs and `all` is already the slow sweep), or
add a `ci-full` recipe that is `ci` plus the instruments and coverage job
legs; then re-state justfile:12-14 so `ci` names only the ci job and `all`
names what it actually covers. The `before` meter failure is a separate
question for its owner (see open questions). Acceptance: at a commit where
CI's coverage job is red, `just all` is red on the same leg.

### verification-infra-3: The rumors mutation campaign is a hand run with no recipe, no confirming re-run, and no kill-state record since 02560b1f
- Where: .cargo/mutants.toml:26-29 (related: justfile:314-326, tools/mutantcheck:21-23, tools/mutantcheck-expected.json:1-61, .agent-notes/2026-08-21-cbor-wire-mutants-scopeA/README.md:40-41 and 202, justfile:1-2)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the roster header, the checker header, every `cargo mutants` line in the justfile, the scope-A campaign note; mapped every roster entry's file to its crate: `accumulator.rs` is suanpan, `grow.rs`/`integral.rs`/`prescan.rs`/`watermark.rs` are before; zero entries name a rumors file)
- Verification: reframed — the sweep said rumors has "no committed campaign outcome". A campaign did run (scope A, 3141 mutants, 126 survivors, cargo-mutants 27.1.0, ox-east-1, ~6 hours) and its survivors were disposed on the w2/mutant-coverage branch and recorded (4c70e958, 1a7afe1e). What remains true: no recipe names the campaign, the note's own first next step (the confirming re-run at the branch tip) has no recorded outcome, scope B never ran, and the tree under test was 02560b1f — before the V1 retirement and the codec rewrite. History: already-known (the note lists the re-run as outstanding).
- Owner-gated: yes — a campaign is hours of compute; its cadence and where its record lives are owner decisions

The roster header names `cargo mutants --workspace` as the campaign
configuration of record, but the justfile runs only `mutants-list` (which
its own comment labels list-only, never a campaign), and `tools/mutantcheck`
states in its header that no gate leg runs cargo-mutants at all. The
justfile's opening totality claim ("Every artifact in the workspace has a
recipe here") does not cover the campaign. The suite's kill power over the
streaming codec, rewritten since the only campaign, is unmeasured.

Evidence:

    .cargo/mutants.toml
        26	# Campaign configuration of record — the keys below make the plain
        27	# invocation (`cargo mutants --workspace`) run it: nextest, all
        28	# features, the whole workspace's suites, under cargo's default
        29	# dev/test profile. That is what the gate's test leg (`just test-all`)

    tools/mutantcheck
        21	silently swallows every NEW mutant the function later grows — both rot
        22	invisibly because no gate leg runs cargo-mutants at all. This checker
        23	closes that seam from two captured `--list` runs (never a campaign):

    justfile
       314	# Hold the mutants exclusion roster to its pinned counts (list-only, never a campaign).

    .agent-notes/2026-08-21-cbor-wire-mutants-scopeA/README.md
        40	verified present by name at 62447263. What is *not* verified here: that the
        41	landed tests actually kill their mutants under mutation — that needs the
       202	1. **Confirming re-run at the branch tip.** Every "killed by landed test"

Resolution: add a `mutants` recipe that runs the configuration of record
for one package (`cargo mutants -p rumors` with the roster applied) as a
named hand-run outside `gate`/`ci`/`all`, with its output location and the
tool-version pin stated in the recipe comment; run it once at HEAD to
discharge the outstanding confirming re-run; triage every survivor into a
test, a refactor, or a roster entry with rationale (never a committed
MISSED list); amend the justfile's totality claim to name the campaign as
the hand-run it is. Acceptance: a recipe exists, and a dated campaign
record at a named commit shows zero untriaged survivors in rumors.

### verification-infra-4: The coverage pin's scope is before's skyline kernel only; rumors is instrumented on every CI run and never judged
- Where: tools/covcheck-expected.json:2 (related: tools/covcheck:209-214, justfile:1006-1009, justfile:1034)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the expectation file's `scope`, the checker's single-scope `run`, and the coverage recipes)
- Verification: confirmed; history: no-rationale-found (no `.agent-notes/` entry or justfile comment rules rumors out of scope)
- Owner-gated: yes — curating a rumors scope is owner-directed work

`tools/covcheck` reads one `scope` string and rejects any expected entry
outside it; the committed scope is `crates/before/src/version/skyline/`.
The coverage legs run `cargo llvm-cov nextest --workspace --all-features`,
so every rumors kernel — the streaming codec's decode arms, the proxy state
machine, the materialized work queues, the bookmark format — is
instrumented on every CI run and the report is discarded. None of rumors'
`unreachable!`/`expect` arms is held to a coverage residue, so an arm
reachable from wire input but unexercised reads the same as a dead one.

Evidence:

    tools/covcheck-expected.json
         2	 "scope": "crates/before/src/version/skyline/",

    tools/covcheck
       209	    scope = expected["scope"]
       210	    files = parse_lcov(lcov_text, scope)
       211	    entries = expected["branch" if branch else "line"]
       212	    for relpath in entries:
       213	        if not relpath.startswith(scope):
       214	            return [f"{relpath}: expected entry outside the pinned scope {scope!r}"]

    justfile
      1006	# GOAL: no skyline-kernel arm goes silently unexercised — every uncovered
      1034	    {{ justfile_directory() }}/tools/memwatch cargo llvm-cov nextest --workspace --all-features --lcov --output-path target/llvm-cov/workspace.lcov

Resolution: let `covcheck` accept a list of scopes (or a second expectation
file) and curate a rumors scope, starting with
`src/tree/mirror/streaming/remote/` and `src/bookmark/format.rs`; the CI
wall-time cost is zero because the instrumented run already covers the
workspace. If the owner rules rumors out of scope for now, say so in the
justfile's coverage section so the omission reads as a decision.
Acceptance: `covcheck-expected.json` carries a rumors scope whose entries
resolve, and a new uncovered line in that scope reds the coverage job by
name.

### verification-infra-5: No fuzz target exists for any rumors decoder; the spec of record is still unimplemented
- Where: design/rumors-frame-fuzz.md:3 (related: design/rumors-frame-fuzz.md:273-288, justfile:365-368, justfile:549-559, .agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:1562-1564)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the design doc's status line and integration section; `ls fuzz` fails; the justfile's `fuzz-build` and `fuzz` recipes name only `crates/before/fuzz` and its five targets; the doc was last touched today, 3327a92b, so it is maintained as a spec)
- Verification: confirmed; history: already-known (the 2026-07-22 adversarial-resource note records the deferral: "Frame-level fuzzing of `rumors` stays deferred to a future campaign; its spec ... is the record that campaign resumes from")
- Owner-gated: yes — the doc's five open questions await rulings, and the work is a campaign

There is no `fuzz/` workspace beside `src/`, and the justfile's fuzz
recipes build and run only before's targets. The crate's malformed-wire
coverage is hand-crafted point suites one frame at a time; coverage-guided
search over composed bytes at the public session entry is the missing
complement, and the model framing (conformance bug detector, not a
security boundary) is already written down in the doc. Two decoders are
not reachable from the session entry the doc names and would need their
own targets: the bookmark record format (`src/bookmark/format.rs`, reached
through a `BookmarkIo` load) and the routed link header
(`src/link/routed/header.rs`, reached through the routed acceptor).

Evidence:

    design/rumors-frame-fuzz.md
         3	Status: spec of record, not yet implemented (2026-07-27).
       273	- **Location:** a detached fuzz workspace at `fuzz/` beside `src/`,
       274	  mirroring `crates/before/fuzz`: empty `[workspace]` table so the
       275	  stable-toolchain gate never builds it, `cargo-fuzz` package metadata,

    justfile
       365	[working-directory("crates/before/fuzz")]
       366	fuzz-build:
       367	    cargo fmt --check
       368	    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

    $ ls fuzz
    ls: fuzz: No such file or directory

Resolution: rule on the doc's section 7 questions, then implement the
session-level target per section 6 (detached `fuzz/` workspace,
`fuzz_session`, seeds derived from `LinkCapture` with a byte-identity gate
test, fuel and heap caps measured then pinned), wire it into `fuzz-build`
(gate) and `fuzz` (`all`), and add a target pair for the bookmark record
decoder and the routed header decoder. Acceptance: `just fuzz-build`
compiles a rumors target and `just fuzz` runs it for `fuzz_smoke_secs`.

### verification-infra-6: The crate doc's operating-envelope figures have no committed derivation or enforced measurement
- Where: src/lib.rs:31-51 (related: README.md:35-57, tests/dispute_wire.rs:1-31, results/fit_gossip_grid.py:5, results/mirror-complexity.tex:3-4 and 54)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: verified (read src/lib.rs:20-60; grepped `Gb/s`, `Mb/s`, `messages/s` across src, tests, benches, examples, design, results, .agent-notes — hits only in src/lib.rs and its derived README; read tests/dispute_wire.rs's module doc; read the results/ headers and grepped them for BLAKE3 and the retired file names)
- Verification: confirmed for the in-scope claim; the results/ observations are outside this review's stated scope and are moved to the open questions. History: no-rationale-found in scope (commit 4bbd6c5b3's ruling that a dated design record keeps its original vocabulary was made for `design/streaming-latency-serialization.md`, not for `results/`)
- Owner-gated: yes — the doc's wording is the owner's

The crate doc states numeric bounds (10,000 messages/s, 1 Gb/s, 100
messages/s at 10 Mb/s, 80 Mb/s per KB of body, staleness roughly the
square of the bandwidth shortfall) and labels one of them "derived". No
committed test or constant computes any of them. The closest instrument is
`tests/dispute_wire.rs`, which pins the exact per-message wire law at
three cells and is not tied to the doc's figures. The only derivation
artifact is `results/`, which is dated, BLAKE3-denominated, and cites two
removed files (`results/ANALYSIS.md`, removed in b2ea50bb9 as an
accidentally committed LLM output; `results/mirror-complexity.md`, retired
in b29c4e0e1).

Evidence:

    src/lib.rs
        31	//! - peers produce in total **less than 10,000 messages/second**, and
        32	//! - each peer-to-peer link offers **1 Gb/s or better**.
        48	//! in proportion to roughly the square of the bandwidth shortfall (derived: a
        49	//! session's metadata amortizes, falling roughly as the inverse square root
        50	//! of the backlog at realistic scales, so equilibrium repays a bandwidth
        51	//! deficit with its square in staleness).

    tests/dispute_wire.rs
         3	//! The closed form in `Peer::sync_memory_budget`'s docs takes a
         4	//! session's mean encoded record size through the per-message wire law
         5	//! pinned here, and the design-record anchor (`DISPUTE_WIRE_BYTES`) is

    results/fit_gossip_grid.py
         5	the two-regime model documented in `results/ANALYSIS.md`. Prints the fitted

Resolution: derive the doc's figures in a committed test from the pinned
per-message wire law (the `tests/dispute_wire.rs` constants) plus stated
assumptions (fan-out, rounds per second), and have the doc cite the
constant and its validity band; the owner decides the wording. The
disposition of `results/` (re-denominate against SHA3-256 with its
references restored, move its derivation into the test, or excise) is an
owner call recorded under open questions. Acceptance: a test names each
figure in src/lib.rs and fails when the wire law moves it out of band.

### verification-infra-7: `ci` omits manifestlint, which the gate runs
- Where: justfile:1000 (related: justfile:403, justfile:206-209, .github/workflows/ci.yml:107-108)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (compared the two recipe lines; `git log -L` on both; manifestlint landed 2026-08-31 in 84044759, the `ci` line was last edited 2026-08-20 in de9e0bdf, and 84044759's message reports only "just gate clean")
- Verification: confirmed; history: no-rationale-found (an omission at landing, not a decision)
- Owner-gated: no

A pull request that adds a member-local version, path, or git source to a
manifest passes CI; the check exists only on the committer's machine. The
justfile header describes `ci` as adding what the gate does not reach, not
as dropping a gate lint.

Evidence:

    justfile
       403	gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check
      1000	ci: fmt-check doclint testdoc workflowlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz

Resolution: add `manifestlint` to the `ci` recipe line; it is build-free
and python3 is already a CI prerequisite. Acceptance: `just ci` fails on a
scratch manifest edit that restates a workspace version.

### verification-infra-8: CI installs cargo-mutants unpinned while the count pin fails on any tool version but 27.1.0
- Where: .github/workflows/ci.yml:76-86 (related: tools/mutantcheck-expected.json:2, tools/mutantcheck:75-79, justfile:308-311)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read ci.yml, the expected file, and the checker's provenance paragraph; git history: the install line landed in e4d92ae4 and the tool-version pin in bec1ceaf afterwards, and the install comment was not revisited)
- Verification: confirmed; history: deliberate-but-expired (the comment's rationale — only cargo-rdme produces a committed artifact — was true when written and is false since bec1ceaf)
- Owner-gated: no

The install step's comment says cargo-rdme is the only tool whose output is
a committed artifact and therefore the only one pinned. `tools/mutantcheck`
also pins a tool version string and fails on mismatch by design. Because
`cargo-mutants` rides the pinned install-action's manifest, a Dependabot
bump of that action can move cargo-mutants and red `ci` on an untouched
tree — the exact failure the comment says the cargo-rdme pin avoids.

Evidence:

    .github/workflows/ci.yml
        76	      # cargo-rdme carries a version because it is the only tool here whose
        77	      # output is a committed artifact compared byte for byte: readme-check
        78	      # holds the READMEs against what it emits. Tracking the newest release
        86	          tool: just,cargo-nextest,cargo-rdme@2.1.0,cargo-fuzz,cargo-mutants,wasm-pack

    tools/mutantcheck-expected.json
         2	  "tool": "cargo-mutants 27.1.0",

    tools/mutantcheck
        75	Provenance: the counts derive from the installed cargo-mutants release
        76	(its operator set and name spellings move between releases), so the
        77	expected file pins the `--tool-version` string and a mismatch is a
        78	finding, never a silent re-baseline. Bumping the tool re-pins the counts

Resolution: pin `cargo-mutants@27.1.0` in the install step and rewrite the
comment: two tools carry versions because two committed artifacts depend
on them, and bumping either is a reviewed diff that re-pins
`tools/mutantcheck-expected.json` or the READMEs in the same commit.
Acceptance: `tools/workflowlint` still passes and the install line names
both pins.

### verification-infra-9: testdoc's `.` walk reads untracked trees, including another agent's worktree
- Where: tools/testdoc:19 (related: tools/testdoc:72-78, justfile:184-186, justfile:181, .git/info/exclude:7, tests/seed_liveness.rs:35)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the tool; `find .claude -name '*.rs' | wc -l` reports 543; `.git/info/exclude` line 7 is the only exclusion of `.claude/worktrees/` — `.gitignore` has none; `tests/seed_liveness.rs` skips `.claude` and `doclint` takes explicit roots). The sweep reports running the tool on that worktree with exit 0; I did not reproduce that.
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The gate's testdoc leg runs `./tools/testdoc .` and the tool skips only
`.git`, `node_modules`, and `target`. The repository root holds
`.claude/worktrees/agent-a0e01f4cff55d1ec9/`, a full worktree with 543
`.rs` files, so the gate's verdict is a function of another tree's
uncommitted state. The sibling sweep `tests/seed_liveness.rs` already
skips `.claude`; `doclint` avoids the problem by taking explicit roots.

Evidence:

    tools/testdoc
        19	IGNORED_DIRECTORIES = {".git", "node_modules", "target"}

    justfile
       181	    ./tools/doclint benches crates examples src tests
       184	testdoc:
       185	    ./tools/testdoc --self-test
       186	    ./tools/testdoc .

    .git/info/exclude
         7	**/.claude/worktrees/

    tests/seed_liveness.rs
        35	const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules", ".claude"];

Resolution: give testdoc the same explicit roots doclint uses
(`./tools/testdoc benches crates examples src tests`), or have it walk
`git ls-files '*.rs'`; add `.claude` to the ignore set as a second guard
and a self-test case for the ignore list. Acceptance: an undocumented test
placed under `.claude/worktrees/` does not change `just testdoc`'s verdict.

### verification-infra-10: testdoc cannot see a `proptest!` block test that lacks an explicit `#[test]`
- Where: tools/testdoc:20-23 (related: tools/testdoc:59-63, tools/testdoc:81-103, src/tree/tests.rs:206-217)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (constructed: a scratch file under my scratchpad with an undocumented `proptest!` fn and an undocumented `#[test]` fn yields exactly one finding, the unit test; a Python scan over src, tests, examples, benches counted 118 depth-1 `proptest!` block fns, 0 without `#[test]`, 0 without `///`)
- Verification: confirmed; history: no-rationale-found (the tool's header states the lexical design; the block form is not among its self-test cases)
- Owner-gated: no

The checker keys on a test attribute line. A `proptest! { fn name(x in
strategy) {...} }` item is a test (the macro emits `#[test]`), but its
source carries no attribute unless the author writes one, so the gate
never checks it. Today the class is latent: every block fn in the tree
carries an explicit `#[test]` and a `///` doc, but nothing enforces that
convention, and the property tests are the ones whose invariant statements
matter most.

Evidence:

    tools/testdoc
        20	TEST_ATTRIBUTE = re.compile(
        21	    r"^\s*#\[\s*(?:test|tokio::test|async_std::test|rstest|test_case)"
        22	    r"\s*(?:\([^]]*\))?\s*\]\s*$"
        23	)

    scratch run (file: <scratchpad>/final-sweep-verification-infra/testdoc-demo/demo.rs)
        .../demo.rs:9: test `undocumented_unit` is missing a `///` doc comment
        exit=1
    (the `proptest!` fn at line 4 of the scratch file was not reported)

Resolution: track `proptest! {` blocks lexically and treat each depth-1
`fn` inside as a test entry point; add both the block form and the
omitted-attribute case to `--self-test`. Acceptance: the scratch file
above yields two findings.

### verification-infra-11: tools/digestshare keeps a V1 side-by-side skip for a render form that no longer exists
- Where: tools/digestshare:20-23 (related: tools/digestshare:43-49, tools/digestshare:76-78, justfile:211-220)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read the tool; `grep -l '│' tests/snapshots/*.snap` matches 0 of 22 files; the only `V1` token in src is the unrelated `Protocol` example at src/lib.rs:272; the tool's git history has two commits, neither the V1 retirement (368da2a5) nor the prose re-denomination (c13c21b4))
- Verification: confirmed; history: no-rationale-found (the V1 retirement's re-denomination commit did not touch tools/)
- Owner-gated: no

The tool documents and implements skipping snapshots that contain the `│`
column separator as "V1 side-by-side timelines". No committed snapshot
contains that character and the protocol has no V1; the branch is dead and
its prose is a ghost reference to the retired dialect.

Evidence:

    tools/digestshare
        20	V1 side-by-side timelines (renders containing the `│` column separator)
        21	are skipped and reported as such: their transcripts show each byte twice
        22	(sent and received), so totaling them would double-count; the V1 digest
        23	atom is the same `Hash` the V2 corpus measures.
        48	    if "│" in body:
        49	        return None
        76	            if measured is None:
        77	                print(f"{path.name}: skipped (V1 side-by-side timeline)")
        78	                continue

Resolution: delete the `│` skip, the `measure` docstring's "or None" clause,
and the header paragraph; keep the liveness failure (a corpus with captures
but zero parsed bytes) as the tool's only non-threshold exit. Acceptance:
`grep -n 'V1\|│' tools/digestshare` is empty and `just digestshare` passes.

### verification-infra-12: ci.yml comments describe toolchains and a bench compile the tree no longer has
- Where: .github/workflows/ci.yml:54-67 (related: .github/workflows/ci.yml:17-19, 122-133, 202-216; justfile:40; rust-toolchain.toml:29; Cargo.toml:96-106)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the files; no bare `+nightly` exists in the justfile; the CI log at HEAD shows `info: syncing channel updates for nightly-2026-06-30-x86_64-unknown-linux-gnu` at the doctest step and `the toolchain '1.97.1-x86_64-unknown-linux-gnu' is currently in use (overridden by '/home/runner/work/rumors/rumors/rust-toolchain.toml')` at the stable install step; history: the nightly comment dates from 788b5366f when `nightly_toolchain := "nightly"`, pinned dated in e7a4b7b0c; the lib-as-bench comment (89369ed47) predates `bench = false` (67ac9846e))
- Verification: confirmed; history: deliberate-but-expired on all four points
- Owner-gated: no

Four claims in the workflow's comments are contradicted by the justfile,
Cargo.toml, and the runner's own log: (a) "The doctest and fuzz recipes
invoke `cargo +nightly`" — every nightly leg invokes `+nightly-2026-06-30`,
which rustup provisions itself at first use, so the installed floating
`nightly` is unused and the instruments job's "tracks nightly, so a format
bump upstream can turn the leg red" cannot happen under the pin; (b) "a
current stable toolchain (edition 2024 needs 1.85+)" — rust-toolchain.toml
pins 1.97.1 and overrides the installed stable; (c) "the rumors
bench/release tier, whose lib-as-bench compile is known to exceed 16 GiB"
names a compile that `[lib] bench = false` removed; (d) the coverage job
installs `llvm-tools` on floating nightly, not the pinned nightly the
branch leg runs (cargo-llvm-cov provisions the component itself, so the
step is inert rather than wrong).

Evidence:

    .github/workflows/ci.yml
        56	      # The doctest and fuzz recipes invoke `cargo +nightly`, so nightly only
        57	      # needs to exist, not be the default — whereas the gate's bare
        58	      # `cargo fmt`/`clippy`/`check` must hit stable.
       124	  # never the rumors bench/release tier, whose lib-as-bench compile is
       125	  # known to exceed 16 GiB; that build lives in `ci`'s bench-build leg,
       130	  # format_version. This job tracks nightly, so a format bump upstream can
       131	  # turn the leg red on an untouched tree — the checker refuses loudly,

    justfile
        40	nightly_toolchain := "nightly-2026-06-30"

    rust-toolchain.toml
        29	channel = "1.97.1"

    Cargo.toml
       106	bench = false

Resolution: install the pinned nightly by name (read `nightly_toolchain`
from the justfile in a step, or drop the nightly install and rely on
rustup's provisioning as AGENTS.md describes), rewrite the four comments to
today's mechanism, and either install `llvm-tools` on the pinned nightly or
state that cargo-llvm-cov provisions it. Acceptance: no comment in ci.yml
names a floating `nightly` as the toolchain a recipe uses, or a lib bench
compile.

### verification-infra-13: `just --list` shows mid-sentence fragments for eight recipes
- Where: justfile:111-114 (related: justfile:287-289, 314-321, 363-366, 588-594, 633-641, 705-710, 792-807)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read the comment layout above each recipe; just takes the comment line immediately preceding a recipe, or its attributes, as the listing description; I did not run `just --list`, and the sweep's reported output is unverified corroboration)
- Verification: confirmed from layout; history: no-rationale-found
- Owner-gated: no

Where a multi-line explanatory block abuts the recipe with no blank line
and no one-line summary, the tour shows the block's last line. The eight
recipes are `test-all` ("module build only here)."), `citecheck`
("collected test inventory."), `mutants-list` ("capture, and this flag is
what keeps that refusal from ever firing."), `fuzz-build` ("workspace's
formatting leg: the root `cargo fmt --all` cannot reach it."), `fuzzfit`
("surfacecheck recipes carry the same discipline)."), `wasm32-pins`
("gates — the fuzzfit recipes carry the same discipline)."), `doc-figure`
("that lets the build write into the source tree."), and `bench-alloc-ab`
("(never quoted)."). The header calls `just --list` "the tour".

Evidence:

    justfile
       111	# The gate's test run: every feature (the meter suites and the conformance
       112	# module build only here).
       113	test-all *args:
       805	# Reduced-sampling smoke: append `--sample-size 10 --measurement-time 1`
       806	# (never quoted).
       807	bench-alloc-ab target arm="shipped" *filter:

Resolution: for each of the eight recipes, end the explanatory block with a
blank line and add a one-line summary comment directly above the recipe
(or its attribute line), the pattern the other recipes already follow.
Acceptance: every line of `just --list --unsorted` reads as a complete
sentence.

### verification-infra-14: tradeoff_probe is a hand-run instrument with no recipe, against the justfile's totality claim
- Where: tests/tradeoff_probe.rs:186-188 (related: tests/tradeoff_probe.rs:1-10, justfile:1-3, justfile:1002-1003)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the module doc and attribute; `grep -n 'tradeoff_probe\|run-ignored\|ignored' justfile` is empty; the only other `#[ignore]` in the tree is disruption.rs's child-process entry point, which is not an instrument)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The instrument is `#[ignore]`-gated and its invocation lives only in its
module doc; no recipe names it although the justfile opens by claiming
every artifact has one. Its counts are deterministic (virtual time), so it
qualifies for `all` cadence, and its predictions (the solve-derived wave
form the committed trade-off table tabulates) can drift from the measured
sessions with nothing noticing.

Evidence:

    tests/tradeoff_probe.rs
         1	//! One-shot validation instrument, ignore-gated: the solve-derived
         2	//! trade-off predictions held against measured wire-time slowdowns.
         9	//!     cargo nextest run --release --test tradeoff_probe \
        10	//!         --run-ignored all --no-capture
       186	#[test]
       187	#[ignore = "one-shot validation instrument: run explicitly with --run-ignored"]
       188	fn tradeoff_closed_form_validation_run() {

    justfile
         1	# rumors workspace: the source of truth for verification. Every artifact in
         2	# the workspace has a recipe here, tiered by feedback speed, and `just --list`

Resolution: add a `tradeoff-probe` recipe wrapping the documented command
and include it in `all`, or amend the justfile's totality claim to name the
hand-run exceptions. Acceptance: `grep tradeoff justfile` names the recipe
and the module doc points at it instead of a bare command.

### verification-infra-15: AGENTS.md's snapshot re-accept witness is stated in terms of hexdump lines the corpus no longer has
- Where: AGENTS.md:155-161 (related: tests/snapshots/*.snap, src/tree/mirror/streaming/remote/codec/capture.rs:12, tools/digestshare:15-18)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the clause; every snapshot is a CBOR reflection render since ef6569c4, 2026-08-19; a grep for hexdump-shaped lines over `tests/snapshots/*.snap` matches nothing; the clause was written 2026-08-13 in c6333e9e; ef6569c4's message states the render change "does not qualify under, and does not use, the hex-line-preservation class")
- Verification: reframed — the sweep said the witness is a convention with no mechanical check; the prior problem is that the witness cannot be computed as worded, because the corpus has no hexdump lines. History: deliberate-but-expired
- Owner-gated: yes — re-defining a sanctioned re-accept class is policy

AGENTS.md sanctions a renderer-vocabulary re-accept only with "the
hex-line-preservation witness — every hexdump line sequence identical to
the parent commit". The snapshots are now fully unfolded CBOR value trees
whose byte pin rests on injectivity plus the exact byte-count headers
(`control item N (B bytes)`, `..., B wire bytes`) and `h'…'` literals;
there is no hexdump line to preserve. The clause is a ghost reference, and
the class it defines has no computable witness and no mechanical check.

Evidence:

    AGENTS.md
       155	  One further sanctioned re-accept class: a renderer-vocabulary change
       156	  (the capture renderer's decoded annotations gained or reworded, the
       157	  wire untouched), permitted only with the hex-line-preservation
       158	  witness — every hexdump line sequence identical to the parent commit,
       159	  the diff pure annotation additions or rewordings — and the re-accepting

    src/tree/mirror/streaming/remote/codec/capture.rs
        12	//! # Why a rendering with no hexdump is still a byte pin

    git show -s ef6569c4 (excerpt)
        snapshot extractor consumer, 2026-08-19); it does not qualify under,
        and does not use, the hex-line-preservation class.

Resolution: re-state the witness in today's terms (every byte-count header
and every `h'…'` literal identical to the parent commit, the diff pure
annotation additions or rewordings), and give it a mechanical form — a
small `tools/snapwitness` or a `snap-witness` recipe that extracts those
tokens from `git show <parent>:tests/snapshots/x.snap` and the working copy
and reports identical/moved per file — cited from AGENTS.md and run at
re-accept time. Acceptance: AGENTS.md names no hexdump, and the tool reds
on a scratch snapshot whose byte count moved under an annotation-only diff.

### verification-infra-16: The pipelining hop budget's known-bad regime is stated in prose, not driven by a committed red cell
- Where: tests/gossip_pipelining.rs:33-41 (related: tests/gossip_pipelining.rs:50-51, tests/window_knee.rs:1-16, tests/future_size.rs:26-32)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read the file in full: one `#[test]`, measuring only the production window; read window_knee's module doc and test roster, which does carry the above-knee direction for its own shapes)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The budget's adequacy rests on a figure at the constant (a floor-window
descent pays ≥ ~500 hops, 20× above the budget), but no committed cell
drives the floor window and asserts it reads above the budget. The window
suite is not blind — `window_knee` pins wave growth above its knee — but
this pin lacks its own demonstration that a serialized descent reads red.
`tests/future_size.rs` has the same shape (a budget calibrated against
"a few hundred bytes" in prose), though its known-bad artifact is not
constructible from outside the crate; there the fix is to pin the measured
sizes as constants beside the budget.

Evidence:

    tests/gossip_pipelining.rs
        37	/// never wave costs. A floor-window descent pays one round trip per
        38	/// disputed scope — here ≥ ~250 scopes, hence ≥ 500 hops — so the bound
        39	/// sits 3.4× above the pipelined measurement and the serialized regime
        40	/// sits 20× above the bound.
        41	const HOP_BUDGET: u32 = 24;
        50	#[test]
        51	fn window_pipelines_disputed_scopes() {

    tests/future_size.rs
        28	/// The budget is set generously above the measured sizes (a few hundred bytes)
        29	/// so legitimate growth — an extra captured local, a slightly fatter error type

Resolution: add a floor-window cell to gossip_pipelining (the same
`diverged_pair` built with `sync_window_floor()`) asserting `measured >=
SERIALIZED_FLOOR` with the floor derived from the disputed-scope count, so
the budget's discrimination is measured rather than stated; in future_size,
pin the measured sizes as constants and assert the budget is within a
stated factor of them. Acceptance: the pipelining file carries two cells,
one green under the production window and one asserting the floor window
exceeds `HOP_BUDGET`.

### verification-infra-17: The Changes observer, the routed link, and the handshake state family claims as point tests
- Where: tests/changes.rs:3-6 (related: tests/routed_link.rs:92-100, tests/handshake.rs:1-12, src/link/routed/tests.rs, src/link/routed/header/tests.rs:117-125, src/conformance/link.rs)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (a `grep -c proptest` census over every file in tests/: 33 of 57 suites use none; `src/link/routed/tests.rs` has 15 `#[test]`s and no proptest; the conformance suite under `src/conformance/` uses no proptest, so the routed-link tests that call `rumors::conformance::link::check` run a fixed schedule; only the header codec has a socket-address round-trip property)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Most point-only suites are deliberately point pins (snapshots, meters,
static bounds). Three hold family-shaped claims with points: the Changes
observer pins "exactly one coalesced tick per observed frontier advance
(however many commits that was)" with one test; the routed link layer runs
the conformance suite's fixed schedule and one mesh test, with no property
over connect/accept/label interleavings; the handshake suite pins hand
transcribed preambles. When the claim is a family, stating it as a
proptest invariant is what lets the shrunk counterexample ride along as a
committed seed.

Evidence:

    tests/changes.rs
         3	//! Pins the contract stated on the type: an immediate first yield, exactly
         4	//! one coalesced tick per observed frontier advance (however many commits
         5	//! that was), ticks for every kind of commit — send, redact, and a join
         6	//! learned by gossip — and a clean end once the set closes.

    tests/routed_link.rs
        92	/// Run the whole conformance suite against fresh TCP pairs at the
        94	async fn tcp_conformance(buffers: Option<u32>, dialer_first: bool) {
        97	        rumors::conformance::link::check(async || tcp_pair(buffers, dialer_first).await),

    census: grep -c proptest tests/changes.rs tests/routed_link.rs tests/handshake.rs src/conformance/link.rs -> 0 0 0 0

Resolution: add a Changes property over generated commit sequences and poll
points asserting the coalescing law, and a routed-link property over
generated connect/accept/label schedules against the in-memory network
(the conformance suite as oracle); keep the point tests as named corners.
Acceptance: each file carries a `proptest!` block whose doc states the
family invariant.

### verification-infra-18: A wrong comment path and dated figures in the verification config
- Where: justfile:272-273 (related: .config/nextest.toml:7-10)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`find . -name diff_ops.rs` resolves only `crates/before/src/testing/diff_ops.rs`; read nextest.toml in full)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The citecheck comment names `src/testing/diff_ops.rs`, which at the
repository root resolves to nothing; the file is
`crates/before/src/testing/diff_ops.rs`. `.config/nextest.toml` states a
measured duration and a "today" in a config comment, dated rationale that
rots as the suite changes.

Evidence:

    justfile
       272	# The before coverage roster (crates/before/src/surface.rs) and the bespoke
       273	# half of the pointwise-differential tiling (src/testing/diff_ops.rs) cite

    .config/nextest.toml
         7	# The slowest honest tests today are the proptest suites and the
         8	# inter-process disruption tests, all finishing well under 60 seconds
         9	# (the deterministic fixture search in src/tree/arb.rs is around 6
        10	# seconds), so three 60-second periods —

Resolution: write `crates/before/src/testing/diff_ops.rs`; drop "today" and
the 6-second figure from nextest.toml and derive the budget from the
slow-timeout period alone. Acceptance: both comments read as true against
the tree with no date-dependent number.

## Positives

- `tools/testdoc`'s `--self-test` (lines 81-103) pins eight lexical cases
  including the `//!`, `////`, and interleaved-attribute forms, and a
  missing root is a usage error rather than a clean sweep (lines 119-126);
  the gate leads with the self-test so a checker bug names itself.
- `gate-streams` (justfile:409-500) records an ok/failed marker per stream
  before narrating, and a stream with neither marker fails the gate with
  its partial log replayed; an OOM-killed stream cannot read as a pass.
- `tools/mutantcheck`'s header (lines 47-84) states its model of record
  (net movement per pattern, with the count-preserving-swap residual named),
  its dialect boundary, its dedup rule, its tool-version provenance, and
  its liveness floors — a checker whose limits are written where its
  claims are.
- `tools/covcheck` is tamper-evident in both directions (a new uncovered
  line fails; a stale entry fails until the pin tightens) and refuses a
  report with no branch instrumentation (lines 195-204); the justfile's
  coverage section (1014-1015) states why it is not a global threshold.
- Every `proptest!` block fn in src, tests, examples, and benches (118)
  carries an explicit `#[test]` and a `///` doc, so the convention closes
  testdoc's block-form blind spot today (verified by scan).
- `tests/seed_liveness.rs` skips `.claude` (line 35), the right call that
  testdoc has yet to make.
- `design/rumors-frame-fuzz.md` states the model framing (conformance bug
  detector, not a security boundary) in its first section and was
  re-anchored to the wire change landed today (3327a92b), so it is a
  maintained spec rather than a stale one.
- `tests/dispute_wire.rs`'s module doc (lines 1-36) states exactly what its
  pins establish and what they do not, including the truncation residual
  its negative control bounds.

## Open questions for Finch

1. Coverage job red at HEAD (verification-infra-2): `before::meter
   masked_cmp_hole_envelope` fails only under llvm-cov instrumentation
   (peak heap 1156 B against a 480 B pin; the same test passes in the `ci`
   job at the same commit), and the only change under `crates/before`
   between the last green coverage run (3327a92b) and this red is
   `Cargo.lock`. Is a process-global heap meter meaningful under
   `-C instrument-coverage`, and if not, should the coverage legs exclude
   the meter suites or the meter tolerate instrumentation? A `before`
   question, out of this partition, but it blocks a green main.
2. Does `tests/future_size.rs` pass under the release profile today? It has
   never run in any wired leg (verification-infra-1), so the answer is
   unknown; the first release run may itself be red.
3. Is `results/` in the review's scope? It holds the only derivation
   artifact for the crate doc's headline numbers (verification-infra-6), is
   dated 2026-06-12, BLAKE3-denominated, and cites two removed files
   (`results/ANALYSIS.md`, `results/mirror-complexity.md`). Re-denominate,
   move the derivation into a test, or excise: an owner call.
4. Should the coverage pin's scope extend to rumors' streaming kernel and
   bookmark format (verification-infra-4)? The instrumented run already
   covers the workspace; the cost is curation.
5. The fuzz design doc's five open questions (section 7) still await
   rulings before implementation (verification-infra-5); the 2026-07-22
   note records the deferral, not a schedule.
6. Is `just all` meant to be the full local pre-push ladder? If so it
   should absorb the coverage legs (and perhaps the instruments job's
   legs); if not, the header's "Everything" and "exactly as GitHub CI
   builds them" wording needs narrowing (verification-infra-2).
7. Mutation campaign cadence (verification-infra-3): the scope-A note's
   confirming re-run at the branch tip has no recorded outcome and scope B
   never ran; a campaign is hours on ox-east-1. Named hand-run recipe, or
   a scheduled remote run with its record in `.agent-notes/`?

## Dropped

None dropped outright. Three reframed rather than dropped: the coverage-legs
finding (the recipes exist; the gap is composite membership, severity
lowered to medium), the mutation-campaign finding (a campaign ran and was
disposed by hand; the gap is cadence, the outstanding confirming re-run,
and the absent recipe), and the hex-line witness (a ghost reference to the
retired hexdump render, not merely an unmechanized convention).
