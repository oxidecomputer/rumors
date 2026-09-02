# Sweep suite-economics: Test suite timing and economics

## Method and coverage

The sweep's run of record is one `cargo nextest run -p rumors --all-features`
at 9e5784fb (log: `scratchpad/sweeps/suite-economics/run.log`): 836 tests
across 60 binaries, 22.07 s wall, 2 skipped (`disruption::sim_child` and
`tradeoff_probe::tradeoff_closed_form_validation_run`, both `#[ignore]` by
design), no SLOW, retry, or flaky markers. Load averages were 9.65/7.33/6.99
at run start and 19.09/9.70/7.85 at run end, and the build waited on the
shared build-directory lock, so all timings are indicative.

This final pass re-derived every number it relies on from the sweep's own
artifacts rather than its prose: the 836 per-test rows in `all-times.txt`
sum to 295.142 s; the per-binary table (`per-binary.txt`) has no
`future_size` row; the slowest-test ranking matches the sweep's. Every cited
site was read with line numbers; every call site of
`early_first_child_dispute_pair` was enumerated by grep (15 calls in 10
tests, one more than the sweep counted); the justfile, ci.yml, nextest.toml
and mutants.toml were grepped for release-profile flags; git log and blame
were consulted for arb.rs, future_size.rs, nextest.toml, window_corners.rs
and capacity.rs; and `.agent-notes/` was grepped for recorded rationale
(the height-erasure, item-erasure and parent-placement notes and the
2026-07-23 review packet all bear on findings below).

One of the two permitted test invocations was used:
`cargo nextest run -p rumors --all-features -E 'binary(future_size)'`
reported `Starting 0 tests across 1 binary (59 binaries skipped)` and exited
4 with `error: no tests to run` (log:
`scratchpad/final-sweep-suite-economics/future-size-run.log`; load 4.51
before and after). The second invocation was not needed: nothing else in
dispute is settled by a run that needs no file change.

Not visible to this pass: the fixture search's current winning attempt (no
test prints it), the `[4, 256]` witness variant (needs a file change), and
the per-case runtime construction cost in `tests/disruption.rs`.

## Findings

### suite-economics-1: future_size guardrails compile to zero tests in every committed test run
- Where: tests/future_size.rs:17-20 (related: justfile:113-114, justfile:464, justfile:1000, justfile:1033-1042, .github/workflows/ci.yml:107-108, .cargo/mutants.toml:26-37)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (ran the binary alone under the default profile: 0 tests; grep of every recipe for `--release` and `--cargo-profile`: the only release-profile test runs are the detached fuzzfit and wasm32-pins workspaces at justfile:597 and justfile:644)
- Verification: confirmed; history: deliberate-but-expired. The release run has happened by hand (bf91938a, 2026-08-12, raised the budget 1024 to 2048; `.agent-notes/2026-08-19-height-erasure/README.md:105` records "verified in a release run after step 4"), but no committed recipe runs it, and `.cargo/mutants.toml:26-37` fixes the campaign of record to the dev/test profile, so the mutants campaign cannot observe it either.
- Owner-gated: no (a recipe decision; which option is an open question below)

The three public-future size budgets are gated on the release profile, and
every committed test invocation for this crate (`test-all` in the gate's
workspace stream, the `ci` recipe, both coverage legs, the mutants
campaign) runs the default dev profile, so the binary nextest builds holds
no tests and the guardrail has never fired in the gate or CI. The
regression it exists to catch (the docstring at lines 11-15: a re-inlined
`Levels` chain tripping downstream `recursion_limit`) would go unnoticed.
Principle 2: a status board nothing enforces is decoration.

Evidence:

    17	//! The budget is enforced only in release builds: debug layouts carry
    18	//! additional state, and they are not what users ship.
    19	
    20	#![cfg(not(debug_assertions))]

    113	test-all *args:
    114	    {{ justfile_directory() }}/tools/memwatch cargo nextest run --workspace --all-features {{ args }}

    Starting 0 tests across 1 binary (59 binaries skipped)
    Summary [   0.000s] 0 tests run: 0 passed, 0 skipped
    error: no tests to run

Resolution: pick one. (a) Add a release-profile leg for this one binary to
the gate's workspace stream (`cargo nextest run -p rumors --test future_size
--cargo-profile release`), accepting a release build of rumors and its
dependencies per gate. (b) Drop the cfg and pin a debug-profile budget
measured once: the docstring's own argument is order-of-magnitude growth (a
few hundred bytes against tens of KiB), which is visible under either
profile; keep a release constant beside it only if the release number is
wanted on record. Either way the justfile comment for the leg names this
test as its reason. Acceptance: `just gate` (or `just ci`) reports the three
future_size tests as PASS in its nextest output, and deliberately removing
the `Pin<Box<dyn Future>>` erasure fails that leg.

### suite-economics-2: early_first_child_dispute_pair reruns its 262k-tick geometry search on every call (15 calls across 10 tests, about 1.6 s each)
- Where: src/tree/arb.rs:318-359 (related: src/tree/mirror/streaming/remote/proxy/tests/malformed.rs:67, :97, :134, :168, :199; src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:179, :353; src/tree/mirror/streaming/remote/proxy/tests/greeting.rs:58, :72; src/tree/mirror/streaming/remote/proxy/tests.rs:581)
- Class / severity / confidence: performance / medium / high
- Provenance: verified (read the fixture and every call site; run-log clusters: one-call tests 1.625-1.704 s, two-call tests 3.256-3.434 s)
- Verification: reframed. The sweep counted 13 calls; there are 15: `malformed.rs` calls its `deep_pair` wrapper eight times in five tests (three of them inside `for corrupt_left in [false, true]`), and the sweep missed `phase_invalid_signal_propagates_through_the_full_proxy` (malformed.rs:97, 1.704 s). History: no-rationale-found for the per-call recompute; the comment prices the search but does not say why it runs per call.
- Owner-gated: no

Each call precomputes `ATTEMPTS * STRIDE + leaves` first bytes per side
(2 x 131k SHA3-256 hashes plus version ticks) and scans the windows before
building the one winning pair. The precompute is a deterministic function of
nothing, and it costs about 1.6 s per call in the dev profile (the rumors
crate and the `sha3` dependency compile at opt-level 0; only `before` and
`suanpan` are optimized). Ten proxy tests pay it, five of them twice: about
24 s of CPU per suite pass on a value that never changes. Wall time is
untouched under 16-way parallelism; the cargo-mutants campaign (whole suite
per mutant, `.cargo/mutants.toml:78-80`) and the llvm-cov legs pay it in
full. Under nextest's process-per-test model a `LazyLock` recovers only the
within-test duplicate, so memoization alone does not remove the cost.

Evidence:

    321	    /// prices the fixture. Hashing is deterministic and the winning window
    322	    /// is attempt 1581, so 2048 is exact headroom, not a guess; if hashing
    323	    /// or the leaf encoding ever changes, the search either finds another
    324	    /// window within the budget or fails loudly here.
    325	    const ATTEMPTS: usize = 2048;

    356	    let f_a = firsts(&p_a, ATTEMPTS * STRIDE + LEFT_LEAVES);
    357	    let f_b = firsts(&p_b, ATTEMPTS * STRIDE + RIGHT_LEAVES);

    66	    for corrupt_left in [false, true] {
    67	        let (left, right) = deep_pair();

Resolution: try the known window first. Introduce a `HINT_ATTEMPT`
constant (measured after the SHA3 swap: see suite-economics-3), compute only
that window's firsts (`LEFT_LEAVES + RIGHT_LEAVES` ticks from
`burnt(party, HINT_ATTEMPT * STRIDE)`), evaluate the same geometry
predicate, and fall back to the full scan when it fails. The built-versus-
simulated equality assert and the loud exhaustion failure stay; determinism
is unchanged; the prose number becomes an executed constant. Optionally wrap
the result in a `LazyLock` for the two-call tests. A small unit test that
the hint window satisfies the predicate, and that a wrong hint falls through
to the scan, keeps the fast path checked. Acceptance: the ten proxy tests
drop from 1.6-3.4 s to well under 0.1 s each; the fixture still returns a
pair satisfying the geometry predicate; the fallback path is exercised by a
committed check.

### suite-economics-3: "the winning window is attempt 1581" is a hand-maintained number the SHA3 swap left behind
- Where: src/tree/arb.rs:321-325 (related: src/tree/typed/hash.rs (`PathHash::of`), src/tree/typed/path.rs:35-40 (`Path::for_leaf`))
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read plus git history: d800957e8 on 2026-08-06 changed the number from "attempt 622, so 1024" to "attempt 1581, so 2048" when the version encoding changed; 4f18c347 on 2026-09-01 replaced `blake3::hash` with `Sha3_256::digest` in `PathHash::of`, which `Path::for_leaf` calls, and touched arb.rs only at a doc comment on line 235; arb.rs has no commit since). The current winning attempt itself is unobservable without a probe test, so the exact new value is not verified; that the scanned sequence changed is.
- Verification: new finding, surfaced while disputing suite-economics-2; history: the number has been maintained by hand once already (the 2026-08-06 edit), which is the mechanism by which it now drifts.
- Owner-gated: no

The search predicate reads the first byte of `PathHash::of(version bytes)`
for each candidate leaf; the hash function changed, so the sequence of
first bytes and the first satisfying window changed with it. The suite
passes, so some window under 2048 satisfies the geometry, but the comment's
specific number and the sentence built on it ("2048 is exact headroom, not a
guess") are now unverifiable prose. Principle 5 (no hand-maintained counts:
a number that matters lives where something checks it) and Principle 8 (a
number nothing re-measures is a hypothesis).

Evidence:

    321	    /// prices the fixture. Hashing is deterministic and the winning window
    322	    /// is attempt 1581, so 2048 is exact headroom, not a guess; if hashing
    323	    /// or the leaf encoding ever changes, the search either finds another
    324	    /// window within the budget or fails loudly here.
    325	    const ATTEMPTS: usize = 2048;

    -        PathHash(*blake3::hash(bytes).as_bytes())
    +        PathHash(Sha3_256::digest(bytes).into())

    35	    pub fn for_leaf(version: &Version) -> Self {
    36	        Self {
    37	            height: PhantomData,
    38	            hash: PathHash::of(version.as_bytes()).into(),
    39	        }
    40	    }

Resolution: the same change as suite-economics-2's hint constant, measured
once after the SHA3 swap and checked by code (the search asserts the hint
satisfies the predicate, or a dedicated test fails with "update
HINT_ATTEMPT"). If the hint is not wanted, delete the number and state the
structure instead: the winning window lies inside the budget and exhaustion
fails loudly. Acceptance: no attempt number remains in prose that code does
not check.

### suite-economics-4: 256 proptest cases over finite input spaces of 10 to 32 points
- Where: src/tree/mirror/streaming/tests/faults.rs:115-118 (related: src/tree/mirror/streaming/tests/faults.rs:50-58, tests/party_conservation.rs:308, tests/party_conservation.rs:348-351, tests/redaction.rs:30-34)
- Class / severity / confidence: performance / medium / high
- Provenance: verified (read every generator; `GreetingLie` has exactly the five variants `arb_greeting_lie` lists (faulting.rs:52-75); no `PROPTEST_CASES` override exists in `.config/nextest.toml`, the justfile, or ci.yml, so the default 256 applies; run-log times 10.033 s, 3.598 s, 0.820 s, 1.413 s)
- Verification: confirmed; history: no-rationale-found (the reduced-case blocks elsewhere in the tree carry their reason at the declaration; these four carry none)
- Owner-gated: no

`greeting_lies_classify_exactly` draws from a 5-arm `prop_oneof![Just(..)]`
and `any::<bool>()`: ten distinct inputs, run 256 times, each a full-depth
comb session plus a second baseline session for the two benign lies,
10.03 s, the fifth-longest test in the suite. proptest does not deduplicate,
so exhaustive enumeration is strictly stronger (every point, every run) and
about 25x cheaper. The same shape recurs:
`party_returns_to_baseline_under_sequential_cycles(k in 1usize..=12)` is 12
points whose k = 12 case asserts every smaller prefix inside its own loop
(3.60 s); `party_returns_to_baseline_under_interleaved_cycles` draws a
shuffle of 2 to 4 elements, 2! + 3! + 4! = 32 permutations (0.82 s);
`redaction_propagates_from_any_peer` has 2 + 3 + 4 + 5 + 6 = 20 behavioral
points (`n_peers` 2..=6 times `redactor_idx % n_peers`; `value` does not
enter the property) (1.41 s). About 16 s of CPU for what four exhaustive
loops do in under a second. Doctrine: state a family as a proptest
invariant; a dozen-point finite family is not a sampling problem, and the
docstring's "Every greeting lie classifies exactly ... in both orientations"
is literally satisfiable by enumeration.

Evidence:

    115	    #[test]
    116	    fn greeting_lies_classify_exactly(
    117	        lie in arb_greeting_lie(),
    118	        fault_client in any::<bool>(),

    308	    fn party_returns_to_baseline_under_sequential_cycles(k in 1usize..=12) {

    348	    fn party_returns_to_baseline_under_interleaved_cycles(
    349	        order in (2usize..=4)
    350	            .prop_flat_map(|m| Just((0..m).collect::<Vec<usize>>()).prop_shuffle()),
    351	    ) {

    30	    fn redaction_propagates_from_any_peer(
    31	        n_peers in 2usize..=6,
    32	        value in any::<u64>(),
    33	        redactor_idx in any::<usize>(),
    34	    ) {

Resolution: replace the `proptest!` wrappers with plain `#[test]`
exhaustive loops: `for lie in GreetingLie::ALL { for fault_client in
[false, true] { .. } }` (add an `ALL` const if absent), hoisting the comb
fixture and its baseline sessions out of the loop; a single k = 12 run for
the sequential-cycles test (or `for k in 1..=12` if independent fleets per k
are wanted); `for m in 2..=4` over `Itertools::permutations` for the
interleaved test; `for n in 2..=6 { for r in 0..n }` with one fixed value for
the redaction test. Docstrings stand unchanged. Acceptance: the four tests
keep their assertions and docstrings, cover every point of their input space
deterministically, and finish in under 1 s combined (from about 16 s).

### suite-economics-5: identical generated executions repeated across tests that differ only in their assertion
- Where: tests/multi_peer.rs:40-62 (related: tests/multi_peer.rs:81-85, tests/multi_peer.rs:106-110, tests/sanity.rs:20-29, tests/window_census.rs:124, tests/window_census.rs:273-280, tests/window_census.rs:89-96, tests/window_corners.rs:128-129, tests/window_corners.rs:139-140)
- Class / severity / confidence: performance / low / high
- Provenance: verified (read the four files in full and `execute_and_quiesce` in tests/common/schedule/executor.rs:76-85; `N_PEERS` and `MAX_EVENTS` are identical between multi_peer.rs:21-22 and sanity.rs:15-16; run-log times 3.976, 3.967, 4.170, 4.041 s for the four u64 multi_peer properties, 3.515 s for the sanity property, 3.799 s for the census duplicate, 1.139 s for the corners test)
- Verification: confirmed, with one trade stated that the sweep did not: merging the four multi_peer properties keeps 256 schedules per invariant but reduces the distinct schedules the suite draws per pass from about 1024 to 256. History: no-rationale-found.
- Owner-gated: no

Four `multi_peer` properties run the same `schedule_u64()` and
`arb_window_assignment()` generator through `execute_and_quiesce` at 256
cases each and differ only in what they assert on the result (the string
variant differs in `T` and is a separate claim). `sanity::
arbitrary_schedules_dont_panic` runs the identical generator and executor
and asserts nothing; its docstring's justification ("if this fails, the
others cannot run") is circular, since a panic inside `execute_and_quiesce`
fails every multi_peer test the same way. `window_census::
floor_overhead_is_bounded_by_content` recomputes the `overhead(0,
DIVERGENT_WIDE)` measurement that `window_attributable_residency_stays_
inside_admittance` computes at line 124, and does so by repeating the body
of the `overhead` helper (lines 274-280 against 90-96) because it also needs
`after`. `window_corners::zero_budget_serializes_but_completes` builds the
pair and runs the session twice because `latency::session_hops` consumes
the pair and returns only the hop count. Roughly 20 s of CPU per pass; wall
time is unaffected under nextest parallelism, but cargo-mutants and
llvm-cov pay CPU. Principle 3 for the sanity test; redundant setup for the
rest.

Evidence:

    40	    fn all_peers_converge_after_quiesce(
    41	        schedule in schedule_u64(),
    42	        windows in arb_window_assignment(),
    43	    ) {
    44	        let result = execute_and_quiesce(&schedule, &windows);

    58	    fn readout_matches_oracle_after_quiesce(
    59	        schedule in schedule_u64(),
    60	        windows in arb_window_assignment(),
    61	    ) {
    62	        let result = execute_and_quiesce(&schedule, &windows);

    20	    /// Arbitrary schedules complete without panicking and produce a
    21	    /// finite converged state. The safety net for every other
    22	    /// invariant in the suite — if this fails, the others cannot run.

    273	fn floor_overhead_is_bounded_by_content() {
    274	    let (left, right) = diverged(0, DIVERGENT_WIDE);
    275	    let before = node_census().live;
    276	    node_census_reset();
    277	    reconcile(&left, &right);
    278	    let peak = node_census().peak;
    279	    let after = node_census().live;
    280	    let overhead = peak.saturating_sub(before + after);

    128	    let measured = hops(pair(0, 2_048, divergence, divergence));
    129	    let (left, right) = pair(0, 2_048, divergence, divergence);

Resolution: multi_peer: one generated execution per case with the four u64
invariants asserted by named helper functions (the docstring enumerates
them), keeping the `_at_floor` and string legs as separate distributions;
if the wider draw is worth its cost, say so at the declaration and keep the
split. sanity: delete `arbitrary_schedules_dont_panic` or re-aim it at a
distribution nothing else executes. window_census: have `overhead` return
`before` and `after` alongside the overhead and call it from both tests
(removing the verbatim copy of its body), or assert the floor bound inside
the admittance test's floor leg. window_corners: use
`DelayedWire::round_trip_virtual` once (it returns the reconciled pair and
the elapsed virtual time; `hops_on_lattice` converts the latter). Acceptance:
every invariant currently asserted is still asserted on at least 256
generated schedules; one definition of the census differencing in
window_census.rs; per-pass CPU for these binaries drops by roughly 20 s;
`just gate` verdict unchanged.

### suite-economics-6: nextest.toml describes a wall-clock upper bound no test asserts and cites drifted measurements
- Where: .config/nextest.toml:28-35 (related: .config/nextest.toml:7-12, tests/window_corners.rs:206-212)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both files in full; no wall-clock `<=` assertion exists in window_corners.rs; git: f0762b8b2 on 2026-07-31 made the real-clock corners floors-only, after the nextest.toml sentence was written in a4d2e5460 on 2026-07-24; run-log times for the inter-process test 0.664 s, the fixture search about 1.6 s per call)
- Verification: confirmed; history: the 2026-07-23 review packet (`review-link-transport-branch.md:642` and `:661`) flagged two other sentences in this file, since fixed; the current drift postdates those fixes.
- Owner-gated: no

The trailing paragraph says the window_corners real-clock cross-check
"holds a ~0.5 s measured catch-up under a 5 s budget" and that "~8x
headroom is what absorbs load there"; `real_clock_corners_match_the_
virtual_model` asserts floors only, by its own docstring, and no 5 s budget
exists anywhere in the file. The header paragraph anchors the 180 s
terminate budget on "the inter-process disruption tests" and a fixture
search "around 6 seconds"; the inter-process test now takes 0.66 s, the
fixture search about 1.6 s per call, and the slowest tests are the capacity
witness (19.9 s), the bookmark plans (11.7 s, 10.1 s) and the codec manifest
(11.2 s). Principle 5: prose at a declaration site speaks in the present
tense and carries no hand-maintained measurements; a justification citing a
check that does not exist misleads whoever next tunes the timeout.

Evidence:

    7	# The slowest honest tests today are the proptest suites and the
    8	# inter-process disruption tests, all finishing well under 60 seconds
    9	# (the deterministic fixture search in src/tree/arb.rs is around 6
    10	# seconds), so three 60-second periods —

    32	# of the suite like any other test. The one wall-clock upper bound among
    33	# them (window_corners' real-clock promptness cross-check) holds a
    34	# ~0.5 s measured catch-up under a 5 s budget: that ~8x headroom is what
    35	# absorbs load there, where the virtual pins need none.

    206	/// Both legs assert floors only, because only the never-undercharge
    207	/// direction is load-independent on a wall clock: machine load inflates real
    208	/// elapsed time and can never compress it, so these assertions read the
    209	/// same under any suite parallelism.

Resolution: delete the wall-clock-upper-bound sentence or re-anchor it to
the floor-only claim (a floor cannot flake under load, which is the point
worth stating). Restate the budget's premise without numbers: the
deterministic pollers turn stalls into errors, every committed test finishes
far inside one period, and the terminate budget is a backstop for escaped
hangs rather than a per-test bound. Acceptance: every sentence in the file
describes a check or mechanism that exists in the tree, and no timing
number remains that a reader would need to re-measure to trust.

### suite-economics-7: the capacity witness runs three 8192-leaf sessions; the parent count may not be load-bearing
- Where: src/tree/mirror/streaming/tests/capacity.rs:171-194 (related: src/tree/mirror/streaming/tests/capacity.rs:73-80, src/tree/mirror/streaming/tests/capacity.rs:291-330, src/tree/mirror/streaming/window.rs:132, src/tree/mirror/streaming/materialized/work/queues.rs:93)
- Class / severity / confidence: performance / low / low
- Provenance: assessed (read; not run: the variant needs a file change)
- Verification: confirmed as a question, not a defect; history: already-known observation. The 2026-07-23 review packet recorded the test at 22.6 s as "within bounds, worth watching" (`review-link-transport-branch.md:18`); `.agent-notes/2026-07-18-parent-placement/parent-placement.md:66-68` cites the `[32,256]` pyramid and the 253/254 boundary as "254 = fan − 2", a per-scope property (`FAN = 256`, window.rs:132; `assembly_level_returns` is sized by `FAN`, queues.rs:93). Nothing recorded says why 32 parents rather than the stress matrix's 4.
- Owner-gated: no

At 19.86 s this is the suite's longest test and sets its wall-clock
critical path (everything else completes within the same 22 s on 16
workers). It runs one observed session plus two stall probes on
`pyramid_pair(&[32, 256], 1, LeafOrder::Reversed)`, 8192 disputed leaves
each. Its assertions concern the `AssemblyLevelReturns` high-water mark
(at least 254) and the 253/254 stall boundary, both properties of one
parent's fan; the stress matrix's "recursive full fan" case already reaches
"the fan-sized inter-level return boundary" at `[4, 256]` (lines 73-80),
and `parent_delay_no_cross_parent_backlog` (lines 291-330) pins that return
backlog does not span sibling parents. If the 32 is not load-bearing the
critical path shrinks about 8x; if it is, the reason belongs in the comment,
which states the claim but not why this width.

Evidence:

    173	fn capacity_stress_witness_requires_inter_level_fan() {
    174	    let (a, b) = pyramid_pair(&[32, 256], 1, LeafOrder::Reversed);
    175	    let expected = join_oracle(a.clone(), b.clone());
    176	    let (actual, report) =
    177	        with_observation(|| scheduled_streaming_mirror(a.clone(), b.clone(), vec![2; 16_384]));

    77	    // A full fan below four simultaneously disputed parents reaches every
    78	    // one-slot recursive query/resolution boundary and the fan-sized
    79	    // inter-level return boundary, with a sibling backlog behind it.

Resolution: construct, do not argue. Run the witness once at
`pyramid_pair(&[4, 256], 1, LeafOrder::Reversed)` and check all three
assertions (high water at least 254, stalls at 253, completes at 254). If
they hold, shrink the fixture and add one sentence saying why four parents
suffice; if not, record the mechanism that requires 32. Acceptance: either
the test runs in about 2.5 s with the same three assertions, or its comment
states why the stage must be 32 wide.
Construction: copy the test body with `&[4, 256]`, run it alone
(`cargo nextest run -p rumors --all-features -E
'test(capacity_stress_witness)'`), and compare the report's
`AssemblyLevelReturns` high water and the two `underbuffered_mirror_stalls`
outcomes against the current assertions.

### suite-economics-8: sixty link units for 836 tests: tests/common compiled 42 times, latency.rs seven times
- Where: tests/common/mod.rs:31-33 (related: tests/gossip_pipelining.rs:13-15, tests/hop_trace.rs, tests/latency_link.rs, tests/tradeoff_probe.rs:30-32, tests/window_operator.rs, tests/window_corners.rs:14-16, tests/window_knee.rs; tests/partition.rs, tests/retire_redaction.rs, tests/stale_floor.rs, tests/opening_supply.rs, tests/gossip_pipelining.rs (single-test binaries); proptest-regressions/partition.txt)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep: 42 of the 58 `tests/*.rs` files begin `mod common;`; seven include `benches/support/latency.rs` by `#[path]`; Cargo.toml declares no `[[test]]`, so each file is its own binary; the run log's per-binary table shows five single-test binaries in `tests/` and one two-test window binary; build.log: 32.74 s warm rebuild of the test targets under load)
- Verification: confirmed; history: deliberate-and-holds for the per-category layout (`tests/common/mod.rs:3` "Each per-category test binary pulls this module in via `mod common;`", and AGENTS.md's "category binaries in tests/"); the build cost itself is a recorded owner concern (`.agent-notes/2026-08-20-item-erasure/item-erasure.md:28-31`: "the 24 session-exercising test binaries each re-buying the subsystem"), which item erasure reduced per binary without reducing the binary count.
- Owner-gated: yes: AGENTS.md documents per-category binaries as the intended layout, so consolidation is a layout decision, not a defect.

Every `tests/*.rs` is auto-discovered as its own binary, so nextest built
60 units: 42 include `mod common;` (hence the blanket `allow(dead_code,
unused_imports)`), and seven re-include `benches/support/latency.rs` via
`#[path]`, each a separate compile of that module plus a full link against
the rumors rlib. Five binaries run a single test (gossip_pipelining,
opening_supply, partition, retire_redaction, stale_floor) and
`future_size` compiles to zero tests under the gate. nextest parallelism is
per test, so merging costs no runtime parallelism; the price is compile and
link time on every gate, ci, mutants and coverage build. One cost the sweep
did not state: committed proptest seeds are keyed by source path
(`tests/main.rs` and `tests/seed_liveness.rs` enforce
`proptest-regressions/<suite>.txt`), so folding `partition.rs` into
`multi_peer.rs` moves `proptest-regressions/partition.txt`'s entries into
`multi_peer.txt`, where every seed replays before every property in the
file.

Evidence:

    31	//! Not every binary uses every module; suppress unused-code warnings here
    32	//! rather than peppering allows across modules.
    33	#![allow(dead_code, unused_imports)]

    14	#[allow(dead_code)]
    15	#[path = "../benches/support/latency.rs"]
    16	mod latency;

Resolution: candidates that lose no clarity: fold the window family
(window_census, window_corners, window_knee, window_operator,
window_sweep, gossip_pipelining, tradeoff_probe) into one `tests/window.rs`
with submodules and a single `mod latency;`; fold the single-test files
into their category siblings (retire_redaction into retire, stale_floor and
opening_supply into the handshake or listen category, partition into
multi_peer), moving the committed seeds with them. Leave the schedule-engine
suites as they are: their per-category names are the map AGENTS.md points
at. Acceptance: fewer test binaries in `cargo nextest list -p rumors` with
every test still present by name; latency.rs compiled once for the window
family; `tests/seed_liveness.rs` still passes; `just gate` verdict
unchanged.

### suite-economics-9: ci.yml quotes an 8 GiB memwatch cap; the tool's default is 32
- Where: .github/workflows/ci.yml:26-28 (related: tools/memwatch:46)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read ci.yml:1-60 and tools/memwatch:40-60; grep shows no `PROC_LIMIT_GB` override in ci.yml or the justfile recipes, only the justfile:105 usage hint)
- Verification: confirmed; history: already-known pattern. The 2026-07-23 review flagged `tools/memwatch:35` as "12 GiB comment above a default of 20" (`review-link-transport-branch.md:646`); the default has since moved to 32 and the stale number now sits in ci.yml instead.
- Owner-gated: no

The workflow header states memwatch's per-process cap as 8 GiB; the tool
sets `PROC_LIMIT_GB="${PROC_LIMIT_GB:-32}"` and nothing in CI overrides it.
Principle 5: no hand-maintained counts; state the structure and let the
tool own the number.

Evidence:

    26	# memwatch's swap backstop is sysctl-based and degrades to a no-op off macOS, so
    27	# wrapping the test/doctest/bench recipes is harmless here; its 8 GiB per-process
    28	# cap sits well above anything a normal Linux build reaches.

    46	PROC_LIMIT_GB="${PROC_LIMIT_GB:-32}"

Resolution: drop the figure ("its per-process cap sits well above ...") or
name `PROC_LIMIT_GB` in tools/memwatch as the owner of the number.
Acceptance: the comment states no number the tool can change without
touching the workflow.

### suite-economics-10: inter-process child reaping busy-polls with a 25 ms real-clock sleep
- Where: tests/disruption.rs:798-807 (related: tests/disruption.rs:559-565, tests/disruption.rs:475, Cargo.toml:153-163)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read tests/disruption.rs; grep of `sleep(` across tests/, src/, benches/ and examples/: this is the only real-clock sleep outside examples/swarm.rs's interactive paths; the tokio dev-dependency at Cargo.toml:153-163 enables `rt`, `rt-multi-thread`, `macros`, `io-util`, `net`, `time`, `test-util`, not `process`)
- Verification: confirmed as an idiom point; the cost is bounded (at most 25 ms per child after exit; the whole inter-process test runs in 0.664 s). History: no-rationale-found.
- Owner-gated: no

The parent reaps each child by looping on `try_wait` with a 25 ms
`tokio::time::sleep`. The same tokio dependency can await the child
directly: `tokio::process::Command` with `kill_on_drop(true)` and
`tokio::time::timeout(CHILD_DEADLINE, child.wait())` expresses the same
deadline without polling and dissolves `KillOnDrop`.

Evidence:

    798	        let status = loop {
    799	            if let Some(status) = child.0.try_wait().expect("poll child") {
    800	                break status;
    801	            }
    802	            assert!(
    803	                tokio::time::Instant::now() < deadline,
    804	                "child {index} did not finish within {CHILD_DEADLINE:?}"
    805	            );
    806	            tokio::time::sleep(Duration::from_millis(25)).await;
    807	        };

Resolution: enable tokio's `process` feature in dev-dependencies, spawn via
`tokio::process::Command`, `kill_on_drop(true)`, and
`timeout(CHILD_DEADLINE, child.wait()).await`; the exit-code protocol is
unchanged. Acceptance: no `sleep` remains in disruption.rs; the
inter-process tests keep their exit-code assertions and deadline.

### suite-economics-11: inline #[cfg(test)] test modules where the convention is a sibling tests.rs
- Where: src/tree/arb.rs:672-673 (related: src/testing.rs:396-397, src/tree/mirror/streaming/driver.rs:203-204, src/tree/mirror/streaming/backend/local/adversarial.rs:127-128, src/tree/mirror/streaming/testing/failing.rs:267-268, src/testing/transport.rs:801-802)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep of `#[cfg(test)]` followed by `mod .. {` across src/: six inline test modules against 46 `mod tests;` sibling declarations; `src/tree.rs`'s `meter` and `panic_injection` and `decode.rs`'s `fan_probe` are cfg(test) support modules, not test suites, and are not listed)
- Verification: reframed: the sweep named two sites; there are six. Outside this sweep's lens (noticed while reading the fixture search and `run_to_quiescence`); another partition may hold the same finding. History: no-rationale-found.
- Owner-gated: no

AGENTS.md ("Writing tests") places unit tests in a sibling `tests.rs` via
`mod tests;`, and 46 modules follow it; six carry inline `#[cfg(test)]
mod` blocks instead, one of them named `test` rather than `tests`.
Consistency of where a reader looks for a module's tests.

Evidence:

    672	#[cfg(test)]
    673	mod test {

    396	#[cfg(test)]
    397	mod tests {

Resolution: move the six blocks to sibling `tests.rs` files declared with
`#[cfg(test)] mod tests;`. Acceptance: no inline `#[cfg(test)] mod` test
suites remain in src/; `just testdoc` still passes.

## Positives

- The suite is fast and quiet under load: 836 tests in 22 s wall on a
  machine whose load average rose from 9.6 to 19 during the run, with no
  SLOW markers, retries, or flaky results. The virtual-time discipline in
  `benches/support/latency.rs` (compute costs zero virtual time; assertions
  on the hop lattice) is what lets the window pins pass interleaved with
  everything else, exactly as `.config/nextest.toml:28-32` predicts.
- `.config/nextest.toml:14-24` is a model timeout paragraph: it names the
  one collision the budget knowingly accepts (a shrink phase outrunning
  terminate-after), states why raising the budget is the worse trade, and
  records the recovery procedure (`PROPTEST_MAX_SHRINK_ITERS`,
  `PROPTEST_MAX_SHRINK_TIME`).
- Reduced case counts carry their rationale at the declaration:
  tests/party_conservation.rs:386-392 (population scale as the sampling
  axis), src/tree/mirror/streaming/remote/proxy/tests.rs:524-529 (wide
  generator plus decorator latency, trigger geometry pinned separately),
  src/tree/mirror/streaming/tests/capacity.rs (`arb_stress_widths`:
  structured fan-out without exponential cases). This is the right shape
  for suite-economics decisions, and the finite-space proptests above are
  the sites that lack it.
- Cargo.toml:174-187 optimizes only the two kernel crates in the dev
  profile and explains why debug assertions must stay on (the envelope pins
  count work the `debug_assert!` comparisons perform): a deliberate,
  documented trade of compile time for test time. `.cargo/mutants.toml:26-37`
  states the matching campaign profile and why a release campaign would be
  a weaker observer.
- tests/common/wire.rs:26-32 reuses one current-thread runtime per test
  thread across proptest cases, with the reason stated; tests/main.rs plus
  tests/seed_liveness.rs make proptest seed persistence mechanically
  enforced rather than a convention held in memory.
- tests/disruption.rs's inter-process simulation is cheap (0.66 s for the
  generated cases plus the reconstructed counterexamples) while exercising
  real TCP, real process boundaries, and the exit-code loss protocol; the
  value-oracle tripwires (`value_oracle_tripwires_catch_known_bad_
  mechanisms`) commit the known-bad demonstrations the doctrine asks for.

## Open questions for Finch

1. future_size (suite-economics-1): a release leg for the one binary (a
   release build of rumors and dependencies per gate) or a debug-profile
   budget measured once (cheap; the order-of-magnitude argument holds under
   either profile)? Recommendation: the debug budget, keeping a release
   constant beside it only if the release number is wanted on record.
2. The fixture search (suite-economics-2 and -3): an executable
   `HINT_ATTEMPT` constant converts the prose number into a checked one and
   removes about 24 s of CPU per pass; the alternative is deleting the number
   and leaving the search as it is. Recommendation: the hint, measured after
   the SHA3 swap.
3. The capacity witness (suite-economics-7): does the `[4, 256]` pyramid
   reproduce the high-water mark and the 253/254 boundary? One measured run
   answers it and either shrinks the suite's critical path about 8x or
   produces the sentence the test is missing.
4. Binary layout (suite-economics-8): is the per-category layout meant to
   hold even for the window family's seven `latency.rs` re-inclusions and
   the five single-test binaries, or is consolidating those within the
   intent? Recommendation: consolidate the window family and the
   single-test files, leave the schedule-engine suites.

## Dropped

- Sweep finding [9] (a fresh 16-worker runtime per proptest case in
  disruption): the cost is unmeasured and plausibly small (thread spawn and
  join per case), a fresh runtime per case also isolates any task a faulted
  session leaves behind from the next case, and the construction needs a
  file change; a candidate, not a finding of record.
- Sweep finding [10] (`floor_overhead_is_bounded_by_content` inlines the
  `overhead` helper body): merged into suite-economics-5, whose
  window_census resolution removes the copy.
