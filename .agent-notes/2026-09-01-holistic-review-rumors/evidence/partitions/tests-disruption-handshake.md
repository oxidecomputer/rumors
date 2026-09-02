# Partition tests-disruption-handshake: Integration tests: disruption, gossip_when, pipelining, hop trace, handshake and its liveness

## Partition summary

The partition is six integration binaries, all test code, 3439 lines read in full with line numbers (tests/disruption.rs 1009, tests/gossip_when.rs 982, tests/hop_trace.rs 647, tests/handshake_liveness.rs 431, tests/handshake.rs 276, tests/gossip_pipelining.rs 94). To settle points I also read tests/common/wire.rs, the cited ranges of tests/common/sim.rs, tests/common/fault.rs, tests/common/peer.rs, tests/common/action.rs, tests/reuse.rs, tests/lifecycle.rs, tests/window_corners.rs, tests/session_stats.rs, benches/support/latency.rs and wire.rs, src/testing.rs, src/peer/gossip.rs (the driver's idle select, the epilogue, and Drive's Drop), src/peer/bootstrap.rs, src/tree/mirror/handshake.rs and its tests, src/tree/mirror/streaming/window.rs, src/link.rs, src/network.rs, src/protocol.rs, src/peer.rs, src/tree/typed/hash.rs, src/error.rs, tools/testdoc, .config/nextest.toml, Cargo.toml, and the git history of 368da2a5, 212c6914, 6d48d8dc, 2c73d032, 3327a92b.

disruption.rs drives the plan-based chaos engine in tests/common/sim twice: in-process on a multi-thread runtime with in-memory wires cut at byte offsets, and inter-process by re-executing the test binary as TCP children. Around the two proptests it carries the value-oracle adequacy tripwires, the custody-transitivity regressions, the two-sided pin that derives MAX_CUT from a metered envelope session, and four reconstructed counterexample plans. gossip_when.rs pins the whole contract of the cue-driven driver with hand-fed cue streams (reduction to one-shot gossip, suppression exactness, unconditional probes, transitive relay of content and of a redaction frontier, the clean-end and error terminals, poison fail-fast, poll cancel-safety) plus two proptests (severed connections; chaotic tick and commit interleavings). handshake.rs hand-transcribes the 30-byte preamble and drives one-shot gossip against a fake peer for each rejection diagnosis. handshake_liveness.rs runs nine session shapes over the one-byte in-memory link under the closed-world quiescence poller. gossip_pipelining.rs and hop_trace.rs measure serialized wire hops in exact virtual time over the delayed-pipe link, the latter with a byte-level tracer that prints the critical path.

The quality is high where the doctrine puts the most weight. Known-bad mechanisms are constructed and shown to fail the checks they exist for; the fault range is derived from a meter using the counters the cuts spend, and pinned from both sides; deadlocks in the liveness matrix are witnessed deterministically rather than by wall clock; hop fixtures self-check their shape before any hop arithmetic runs; every test has a substantive doc comment and assert messages say what a failure means. The dominant issues are of three kinds. First, two harness holes: the inter-process parent swallows panics from its serving tasks (a JoinSet never joined, then aborted), and the gate's testdoc never sees `#[pollster::test]`, so nine tests here and 29 crate-wide are unchecked. Second, determinism drift in gossip_when.rs: five negative assertions rest on 100 ms wall-clock windows while the same file already uses the deterministic stall witness three functions away. Third, residue and duplication: two ghost references survive the V1 retirement, a hand-maintained count is stale, the pipelining and hop-trace fixtures are byte-identical while one doc claims otherwise, `send_random` exists in seven copies, and the handshake suite's six fake peers read and discard the bytes its module doc says it is an oracle for. No finding reveals a production bug.

## Findings

### tests-disruption-handshake-1: Moralizing adverbs without a mechanism: genuine(ly), silently, and real as a synonym for actual
- Where: tests/disruption.rs:8-9 (related: tests/disruption.rs:17, 165, 167, 170; tests/gossip_when.rs:11, 156, 157, 163, 186, 227, 263, 270, 314)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (grep -n -i -E 'genuine|silently|\breal\b' over the six files, each site read)
- Seen by: structure-prose [18]; refutation: reframed (keep 'real' where it contrasts with a simulated or hand-driven counterpart); history: no rationale found
- Owner-gated: no

These words carry no mechanism at the listed sites: "genuinely separate OS processes" says nothing that "separate OS processes" does not, "silently lost" names no mechanism of silence, and gossip_when's "real change", "real news", "real session", "real driver" mean "a change since convergence" or "a changes()-fed driver". The uses that contrast with a counterpart stay: "real TCP" against the in-memory wire (disruption.rs:10, 572), "real peer in a different universe" against the fake peer (handshake.rs:14), "real parallelism" against cooperative scheduling (disruption.rs:43), and "real sessions rather than past their end" (disruption.rs:428).

Evidence:

         8	//! - **Inter-process**: peers split across genuinely separate OS
         9	//!   processes — the test binary re-executes itself as each child — over a

       165	/// appending a ledger entry for a message that is genuinely live in the
       166	/// converged fleet. A *dropped insert* — a value the plan sent but the
       167	/// network silently lost — must fail the multiset check; it is simulated

Resolution: Delete or replace at each listed site: "separate OS processes"; "so any failure is a failure of the invariant" (17); "live in the converged fleet" (165); "lost" (167); "the assertions" (170); "a change since convergence" for "a real change" and "a changes()-fed driver" for "a real driver" in gossip_when.rs. Acceptance: none of the listed sites uses the word, or each remaining use names the counterpart it contrasts with.

### tests-disruption-handshake-2: Hand-rolled environment-variable codec for the child-process script
- Where: tests/disruption.rs:484-506 (related: tests/disruption.rs:455-460, 777-785, 889-901; Cargo.toml:69, 144-172)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read)
- Seen by: structure-prose [13]; refutation: confirmed; history: no rationale found (original to 9eadfc68)
- Owner-gated: no

The parent serializes a ChildPlan into six environment variables through four hand-written encode and decode functions, with "-" as the None sentinel and ":" and "," as separators, and the child re-parses them. It is a second grammar for ChildPlan's shape that must track the struct by hand; the doctrine prefers a library over a hand-rolled format. serde_json is a workspace dependency (Cargo.toml:69) but not among rumors' dev-dependencies (144-172); ciborium is already a regular dependency.

Evidence:

       484	fn encode_cut(cut: Option<usize>) -> String {
       485	    cut.map_or_else(|| "-".to_owned(), |n| n.to_string())
       486	}
       487	
       488	fn decode_cut(s: &str) -> Option<usize> {
       489	    (s != "-").then(|| s.parse().expect("malformed cut budget"))
       490	}
       491	
       492	fn encode_fault(fault: &FaultPlan) -> String {
       493	    format!(
       494	        "{}:{}",
       495	        encode_cut(fault.write_cut),
       496	        encode_cut(fault.read_cut)
       497	    )
       498	}

Resolution: Derive Serialize and Deserialize on FaultPlan (tests/common/fault.rs) and ChildPlan, carry `{index, plan}` in one environment variable beside CHILD_ADDR through serde_json (add it to rumors' dev-dependencies) or ciborium, and delete encode_cut, decode_cut, encode_fault, decode_fault and the split-and-parse in child_main. Acceptance: two environment variables in the child protocol (address and plan); no string codec remains in tests/disruption.rs.

### tests-disruption-handshake-3: The inter-process family draws its cuts from the intra-process envelope's range and has no pin of its own
- Where: tests/disruption.rs:526-539 (related: tests/common/sim.rs:104, 286; tests/disruption.rs:422-448, 568-585)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read)
- Seen by: blind-spots [32]; refutation: reframed (whether most cuts miss is unmeasured; the reconstructed plans' cut offsets near 2000 are evidence that sessions reached such offsets when found); history: no rationale found
- Owner-gated: no

arb_child_plan draws every cut through arb_fault, whose range `0..MAX_CUT` (3072) is pinned two-sided against the intra-process envelope session (five peers, tens of unique values) by max_cut_spans_the_envelope_session. The inter-process population is different: at most three seed messages in the parent and five sends per child. No measurement says where MAX_CUT sits relative to a child's bootstrap, sessions, and retirement, so the family has no liveness floor of its own; the intra-process leg's discipline ("re-measure there before touching this number", sim.rs:103) does not reach it.

Evidence:

       526	fn arb_child_plan(faults: bool) -> impl Strategy<Value = ChildPlan> {
       527	    (
       528	        0usize..6,
       529	        arb_fault(faults),
       530	        prop::collection::vec(arb_fault(faults), 1..4),
       531	        arb_fault(faults),
       532	    )

    tests/common/sim.rs
       286	    let cut = prop_oneof![2 => Just(None), 3 => (0..MAX_CUT).prop_map(Some)];

Resolution: Meter one clean child cycle (bootstrap, session, final gossip, retire) against the parent through `fault::metered` over TCP or the in-memory link, and pin a named inter-process bound from both sides as max_cut_spans_the_envelope_session does; draw arb_child_plan's cuts from that bound. Acceptance: a named constant and a two-sided pin exist for the inter-process family, and arb_child_plan draws from it.
Construction: Add a test that runs `child_main`'s clean path against a metered link and prints the per-endpoint byte count; compare to MAX_CUT. Whichever way the comparison falls, the number is currently unknown, which is the gap.

### tests-disruption-handshake-4: The inter-process testdoc counts "four invariants" against a six-item list, and the leg never redacts
- Where: tests/disruption.rs:571-573 (related: tests/disruption.rs:57-74, 512-517, 839-864)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read 57-68, the six numbered invariants; read run_proc_plan 703-865: it asserts the dishonest log empty, probed disjointness, convergence, per-child content survival, and assert_party_invariants, and never calls assert_deletion_honored or assert_value_oracle)
- Seen by: structure-prose [2], api-economics [44]; refutation: confirmed; history: deliberate but expired (accurate at 9eadfc68; 919132dc added invariants 5 and 6 to the intra-process doc without touching this one)
- Owner-gated: no

The doc points at the intra-process list and counts four; the list has six. A reader checking the count finds six and cannot tell which two are meant. The two absent are the ledger checks, and they are absent for a structural reason the doc should state: ChildPlan carries only `n_sends`, so children never redact and the TCP leg exercises neither deletion honoring nor the value ledger. AGENTS.md: no hand-maintained counts; an inaccurate testdoc is a bug in the test.

Evidence:

       571	    /// The same four invariants as the intra-process simulation, with the
       572	    /// fleet split across OS processes gossiping over real TCP sockets
       573	    /// severed at arbitrary byte offsets.

       512	struct ChildPlan {
       513	    n_sends: usize,
       514	    boot: FaultPlan,
       515	    sessions: Vec<FaultPlan>,
       516	    retire: FaultPlan,
       517	}

Resolution: Name the asserted properties instead of counting them: honest errors only, probed disjointness, survivor convergence, and seed fold-join when no hand-off was lost; state that the ledger checks do not run because children keep no ledger and never redact. Separately (owner's call, see open questions) give ChildPlan a redaction step so the TCP leg covers deletion honoring too. Acceptance: no numeral summarizes the list; the doc says why the ledger checks are absent, or they are asserted.

### tests-disruption-handshake-5: The reconstructed-counterexample section speaks in the past tense and states cut geometry nothing maintains
- Where: tests/disruption.rs:589-594 (related: tests/disruption.rs:604-607, 621-624, 672-674)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (git log -S: the tests were introduced at 6d48d8dc, 2026-08-13, and renamed at 2c73d032; the wire was respelled afterwards at 4dd2053c, 3327a92b, and 368da2a5, none of which touched tests/disruption.rs)
- Seen by: structure-prose [19], blind-spots [27]; refutation: confirmed (27's dating corrected: introduced at 6d48d8dc); history: 19 no rationale found, 27 deliberate but expired
- Owner-gated: no

The header frames the constructions as "Historical" seeds that "no longer" replay, which is provenance prose in the tree; the mechanism sentence (a committed seed regenerates through the strategy's cut range, so a range change re-maps its offsets) is present-tense and worth keeping. The four test docs then describe where their byte-offset cuts land ("dies mid-transfer", "cut in both directions", "late in their byte streams", "near the deep end of the fault range the case was found under"). A byte offset is a property of the wire bytes, and the frame layout has changed since the plans were recorded, so those positions are claims nothing maintains: the plans are still deterministic fault plans worth running, but the docs promise a session position they cannot keep. Prose speaks in the present tense; a testdoc must be accurate.

Evidence:

       589	// Historical shrunk counterexamples, preserved as explicit constructions:
       590	// their committed seeds regenerate through the fault strategy's cut range,
       591	// so a range change re-maps the offsets and the seed no longer replays the
       592	// case it pinned. Each test runs the exact plan its seed's shrink recorded,
       593	// under the same invariants as the proptest above. The seed files stay
       594	// committed; these constructions carry the counterexamples themselves.

       672	/// Reconstructed counterexample: three children whose cuts sit near the deep
       673	/// end of the fault range the case was found under, severing sessions and
       674	/// retirements late in their byte streams.

Resolution: Rewrite the header in the present tense ("Shrunk counterexamples as explicit constructions. A committed seed regenerates through the fault strategy's cut range, so a range change re-maps its offsets and the seed replays a different plan; these constructions carry the plans themselves.") and re-denominate the four docs to what is stable: fixed fault plans from shrunk seeds, run under the full invariant battery, with no claim about where the cuts fall. If the original geometry matters, derive the cut from a metered clean run of the same plan (a fraction of the measured extent) so it survives wire changes. Acceptance: no "Historical", "no longer", or "was found" in the section; no doc states a session position a wire-format change could falsify.

### tests-disruption-handshake-6: Bare blocks around single send_all statements
- Where: tests/disruption.rs:707-709 (related: tests/hop_trace.rs:550-552, 572-574; tests/common/sim.rs:687-689)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (git show 212c6914 -- tests/disruption.rs: the `batch(|batch| { ... })` closure was replaced in place inside pre-existing braces)
- Seen by: structure-prose [15], blind-spots [35]; refutation: confirmed; history: no rationale found (mechanical residue of 212c6914)
- Owner-gated: no

The braces scoped a batch closure that 212c6914 replaced with one `send_all` line; they now scope nothing and read as if something were being dropped early.

Evidence:

       707	    {
       708	        seed.send_all(plan.seed_messages.iter().copied()).unwrap();
       709	    }

Resolution: Replace each block with the bare statement at the four sites. Acceptance: no single-statement bare block around `send_all` remains.

### tests-disruption-handshake-7: Serving-task panics are swallowed in the inter-process parent
- Where: tests/disruption.rs:733-765 (related: tests/disruption.rs:819-828; tests/common/sim.rs:466-478)
- Class / severity / confidence: correctness / high / high
- Provenance: assessed (read; the construction below was not run because no file may be modified)
- Seen by: api-economics [36]; refutation: confirmed, with a recommendation to lower to medium because the masked class is confined to TCP serving in one eight-case proptest and four reconstructed plans; history: no rationale found (the abort-and-discard wind-down is original to 9eadfc68)
- Owner-gated: no

The accept task spawns each serving session into a JoinSet that nothing ever `join_next`s; at wind-down the accept task is aborted and its result discarded. A panic inside `handle.gossip` on the serving path therefore unwinds the spawned task, prints to captured stderr, and the test passes: `serve_errors` is not incremented (the increment sits on the `Err` arm, which a panic never reaches), the handle clone drops during unwind so `try_into_peer` succeeds, and a mid-gossip panic leaves the replica unchanged so no post-run invariant can see it. The intra-process engine propagates panics by `.expect`ing each session task's JoinHandle (sim.rs:476-477); the inter-process engine, whose unique coverage is exactly the TCP serving path, does not. I keep the rubric's classification (a harness bug that masks failures) at high; the refutation's narrowing argument is stated so the owner can calibrate.

Evidence:

       733	        tokio::spawn(async move {
       734	            let mut sessions = tokio::task::JoinSet::new();
       ...
       744	                sessions.spawn(async move {
       ...
       754	                    if let Err(e) = handle.gossip(&mut link).await {
       755	                        serve_errors.fetch_add(1, Ordering::Relaxed);

       819	    // Wind down: stop the prober and the accept loop (dropping its
       820	    // `JoinSet` aborts any straggling serve task), then reclaim the
       ...
       827	    accept.abort();
       828	    let _ = accept.await;

    tests/common/sim.rs
       476	    assert_honest_gossip(&task_a.await.expect("session task A"));
       477	    assert_honest_gossip(&task_b.await.expect("session task B"));

Resolution: Hold the JoinSet outside the accept task (or have the task return it), and after the children are reaped and before reclaiming the Peers drain it: `while let Some(result) = sessions.join_next().await { result.expect("serving session task"); }`. Every serving task has completed by then (every child has exited), so the drain is immediate. End the accept loop by closing the listener rather than by abort, or check `JoinError::is_panic` on the aborted task's result. Acceptance: with `panic!("probe")` inserted as the first statement of the serving task body in a scratch copy, inter_process_disruption_upholds_party_invariants and each reconstructed_* test fail; today they pass.
Construction: In a scratch worktree, add `panic!("probe");` at the top of the closure passed to `sessions.spawn` at line 744; run `cargo nextest run -E 'binary(disruption)' -E 'test(reconstructed_child_retire_cut_at_first_byte)'`; observe PASS with the panic message only in captured stderr.

### tests-disruption-handshake-8: The inter-process content check gates on EXIT_CLEAN children only and credits the wrong mechanism
- Where: tests/disruption.rs:849-862 (related: tests/disruption.rs:808-815, 963-971, 984-1005)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read child_main 940-1008: the clean final gossip with `.expect("clean final gossip")` precedes the retire loop; EXIT_BOOT_LOSS and EXIT_UNCERTAIN are returned only after it)
- Seen by: blind-spots [26]; refutation: confirmed; history: no rationale found (gate, final gossip, and comment born together at 9eadfc68)
- Owner-gated: no

Every child runs a clean final gossip before its retirement begins, and a failure there panics (an abnormal exit the parent rejects), so every child that returned any recognized exit code has its sends in a parent cast. The check admits only EXIT_CLEAN children, excluding the boot-loss and uncertain-retire exits although the guarantee holds for them, and those faulted-retire arms are the interesting ones. The comment attributes the guarantee to the retirement's reconciliation; the mechanism that establishes it is the final gossip, as the child's own comment at 963-964 says.

Evidence:

       849	    // Every cleanly-retired child's sends must have survived into the
       850	    // parent's converged content: its final retirement reconciled before
       851	    // the party hand-off, so nothing it published may be lost.
       852	    let live: BTreeSet<u64> = readouts[0].values().copied().collect();
       853	    for (index, child) in plan.children.iter().enumerate() {
       854	        if clean_children[index] {

       963	    // One clean session so everything this child published is home even
       964	    // before the retirement reconciles.
       ...
       970	        cast.gossip(&mut link).await.expect("clean final gossip");

Resolution: Drop the `clean_children` gate (every child that reached a recognized exit code passed the final gossip) and restate the comment: the clean final gossip commits the child's sends before retirement begins, so every child's sends must be live. Acceptance: the loop asserts `live.contains(&child_value(index, s))` for every child regardless of exit code; the comment names the final gossip; the test still passes.

### tests-disruption-handshake-9: Ghost reference to "both protocol implementations" after the V1 retirement
- Where: tests/gossip_pipelining.rs:5-8 (related: src/protocol.rs:15-19)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep 'both protocol' tests: only this line; src/protocol.rs declares the single variant `V2 = 2`; git show 368da2a5 -- tests/gossip_pipelining.rs shows that commit edited this file, removing the `Protocol` import and `.protocol(Protocol::V2)`, and left the sentence)
- Seen by: structure-prose [1], api-economics [41]; refutation: confirmed; history: contradicts the AGENTS.md hard rule (accurate from 818a8707, expired at 368da2a5)
- Owner-gated: no

The module doc says the test asserts the window "through both protocol implementations". Since 368da2a5 there is one; the sentence names code that no longer exists and misdescribes what the test exercises. AGENTS.md hard rule: nothing in the codebase refers to code that no longer exists.

Evidence:

         5	//! instead of tree depth. The window (set through
         6	//! [`Peer::sync_memory_budget`]) is the fix, and this test asserts it
         7	//! end-to-end — from the public knob, through both protocol implementations, to
         8	//! the channels — by gossiping over a delayed-pipe link on a paused-clock

Resolution: Rewrite as "from the public knob, through the streaming mirror, to the channels" (or drop the middle clause). Acceptance: `grep -rn 'both protocol' tests` returns nothing.

### tests-disruption-handshake-10: Two hop instruments measure the same fixture and are never compared; HOP_BUDGET's floor figures are derived, not measured
- Where: tests/gossip_pipelining.rs:33-41 (related: tests/gossip_pipelining.rs:55-61; tests/hop_trace.rs:495-505; tests/window_corners.rs:118-146; benches/support/latency.rs:515-522)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read both fixtures and both assertions; read window_corners.rs:126-138 and window.rs:300-319 for the budget-0 floor equivalence)
- Seen by: structure-prose [4], blind-spots [28], api-economics [40], [42]; refutation: 4 and 40 confirmed, 42 reframed (the floor regime is measured in window_corners at budget 0, which floors every capacity at one, but on a different fixture); history: the loose bound's coexistence with the exact pin is deliberate and holds (HOP_BUDGET's own doc: the headroom "admits a deeper engaged ladder, never wave costs"; 814f07ad tightened 64 to 24 knowing the exact figure), so dissolving the test is an owner question, not cleanup
- Owner-gated: no (the equality and the measured floor are additive; whether the loose bound stays is the open question below)

gossip_pipelining asserts `session_hops(...) < 24` through the bench instrument and hop_trace pins `trace.hops() == 7` through its tracer, on a byte-identical fixture (finding 31); the two readings of one quantity are never equated, so a divergence between `latency::session_hops` and the tracer would go unnoticed. HOP_BUDGET's rationale quotes a floor cost ("≥ 500 hops") and two ratios ("3.4×", "20×") that no committed test measures on this fixture: the floor was measured once in a commit message (818a8707, about 370 hops) and restated as arithmetic. Doctrine: any quantity computable two ways gets a committed comparison, and a criterion's known-bad demonstration belongs on the criterion's own fixture with a measured number.

Evidence:

        35	/// A pipelined descent measures 7 exact hops (the phase ladder's few
        36	/// active levels); the bound's headroom admits a deeper engaged ladder,
        37	/// never wave costs. A floor-window descent pays one round trip per
        38	/// disputed scope — here ≥ ~250 scopes, hence ≥ 500 hops — so the bound
        39	/// sits 3.4× above the pipelined measurement and the serialized regime
        40	/// sits 20× above the bound.
        41	const HOP_BUDGET: u32 = 24;

        55	    let measured = latency::session_hops(LINK_CAPACITY, DELAY, diverged_pair());

    tests/hop_trace.rs
       504	    assert_eq!(trace.hops(), 7, "insertion-shaped session hop count");

Resolution: Once the fixture is shared (finding 31), add one committed equality `latency::session_hops(CAPACITY, DELAY, pair) == trace.hops()` tying the two instruments, and add a floor leg on the same fixture (`sync_window_floor()` both sides) asserting `floor_measured > HOP_BUDGET`; let HOP_BUDGET's doc quote the measured floor figure and drop the hand-derived ratios. Acceptance: one committed equality between the two hop instruments exists; a test fails if the floor drops below the budget; the constant's doc contains no arithmetic over unmeasured numbers.
Construction: Build `diverged_insertions()` in hop_trace, run it through both `traced_session` and `latency::session_hops(CAPACITY, DELAY, ...)`, and assert equality; for the floor, build the same pair with `.sync_window_floor()` and print `session_hops`.

### tests-disruption-handshake-11: `tokio_block_on as block_on` shadows the quiescence poller's name
- Where: tests/gossip_when.rs:49 (related: tests/gossip_when.rs:707, 822; tests/bookmark_attach.rs:17; tests/bookmark_causality.rs:71; tests/bookmark_transmit_window.rs:42; tests/common/wire.rs:34-59)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep -rn 'tokio_block_on as block_on' tests: four suites)
- Seen by: api-economics [48]; refutation: reframed (a four-suite pattern, not a gossip_when one-off); history: no rationale found (rename shim from 83edcd94)
- Owner-gated: no

In tests/common/wire.rs `block_on` is the closed-world poller that turns a stall into a failure and `tokio_block_on` is the reused runtime; wire.rs:44-48 draws the distinction explicitly. Four suites alias the runtime to `block_on`, so `block_on(async { ... })` in their proptests reads as stall-detected when it is not, in exactly the direction the determinism story matters (finding 14).

Evidence:

        49	use crate::common::wire::{bootstrap_fork_async, tokio_block_on as block_on, wire_gossip_async};

Resolution: Import `tokio_block_on` under its own name at the four sites and call it as such. Acceptance: `grep -rn 'as block_on' tests` is empty.

### tests-disruption-handshake-12: Local copies of common::wire helpers in gossip_when.rs and handshake_liveness.rs
- Where: tests/gossip_when.rs:61-65 (related: tests/gossip_when.rs:54, 57, 896; tests/reuse.rs:26, 31, 52-56; tests/handshake_liveness.rs:69-75; tests/common/wire.rs:144-158, 213-218)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read tests/reuse.rs:1-60 and tests/common/wire.rs in full; grep 'fn pair()' tests returns exactly the two definitions)
- Seen by: structure-prose [11], [12]; refutation: confirmed, plus a new nit (gossip_when.rs:896 bypasses the file's own `links()`); history: 11 no rationale found (both copies born a day apart while bootstrap_fork_async already existed), 12 deliberate but expired (gossip_over predates gossip_pair_async, added at 0d48b153)
- Owner-gated: no

gossip_when's `pair()`, `DEADLINE` (10 s), and `LINK_BUF` (64 KiB) are body-for-body the same as tests/reuse.rs:26, 31, 52-56, rationale comments included. handshake_liveness's `gossip_over` is `common::wire::gossip_pair_async` with the link capacity parameterized instead of fixed at LINK_BUF. Duplicated helpers drift independently, and the clean-drain invariant should have one enforcement site. Also, gossip_when.rs:896 builds its link with `rumors::link::memory_with_capacity(LINK_BUF)` directly while every other test in the file uses `links()`.

Evidence:

        61	async fn pair() -> (Rumors<u64>, Rumors<u64>) {
        62	    let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
        63	    let b = bootstrap_fork_async(&a).await;
        64	    (a, b)
        65	}

    tests/handshake_liveness.rs
        69	async fn gossip_over(a: &Rumors<u64>, b: &Rumors<u64>, capacity: usize) {
        70	    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(capacity);
        71	    let (a_out, b_out) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
        72	    a_out.expect("gossip completes on side A");
        73	    b_out.expect("gossip completes on side B");
        74	    assert_control_drained(a_link, b_link);
        75	}

Resolution: Add `seeded_pair_async<T>()` (floor-pinned seed plus fork) and a generous-deadline constant to tests/common/wire.rs and use them from gossip_when.rs and reuse.rs; give wire.rs a `gossip_pair_with_capacity_async(a, b, capacity)` that `gossip_pair_async` calls with LINK_BUF, and delete `gossip_over`. Use `links()` at gossip_when.rs:896. Acceptance: one definition of the seeded pair under tests/; handshake_liveness.rs defines no session-driving helper of its own.

### tests-disruption-handshake-13: Em-dashes in line comments
- Where: tests/gossip_when.rs:178 (related: tests/gossip_when.rs:211, 271, 517, 760, 865; tests/disruption.rs:389, 821; tests/hop_trace.rs:530; tests/handshake_liveness.rs:88, 134, 171)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep -nP '^\s*//[^/!].*(—|–)' over the six files: exactly the twelve sites)
- Seen by: structure-prose [16]; refutation: confirmed; history: no in-repo rule governs `//` comments (the nearest recorded ruling is R76's colons-for-assert-messages); the rule lives in the owner's global doctrine
- Owner-gated: no

Twelve line comments use typographic em-dashes. The owner's doctrine reserves those for rendered prose and asks for colons or semicolons in code comments; rustdoc (`///`, `//!`) is rendered and is not at issue.

Evidence:

       178	    // polling both drivers now yields nothing — no echo session — and the

Resolution: Rewrite the twelve sites with colons, semicolons, or parentheses. Acceptance: the grep above is empty for the six files.

### tests-disruption-handshake-14: Five negative assertions rest on 100 ms wall-clock windows that pass on expiry; the module doc says "no timers anywhere"
- Where: tests/gossip_when.rs:180-184 (related: tests/gossip_when.rs:4-6, 45, 220-224, 283-287, 293-297, 398-402, 519, 552, 560, 596; src/testing.rs:366-394)
- Class / severity / confidence: test-quality / medium / high
- Provenance: assessed (read; mechanism from tokio's Timeout, which polls the inner future before the sleep; run_to_quiescence's coop handling read at src/testing.rs:372-381)
- Seen by: structure-prose [0], blind-spots [24], api-economics [38]; refutation: confirmed; history: deliberate but expired (written at e43edcd7 before any stall detector existed; run_to_quiescence was imported into this file at 459cff1a for one test without converting the negatives)
- Owner-gated: no

The claims "no echo session", "no heartbeat session", "no observer woke", "no when-changed session", and "a converged chain is quiet" are each established by awaiting a 100 ms timeout and asserting it expired. Timeout polls the inner future first, so an erroneous session still in flight at the deadline yields Pending, the sleep fires, and the negative passes: the failure direction is a false green under load, and each site spends 100 ms of real time. The file already proves negatives deterministically with `run_to_quiescence` at 519, 552, 560, and 596 (Err(Quiescence::Stalled) when the future parks with no wake arranged); the detector disables tokio's cooperative budget around the subject, so it is safe inside a tokio test. The module doc's "no timers anywhere" was scoped to cue timing at birth, but with `use tokio::time::timeout` at 45 and five verdicts resting on a timer a reader takes it suite-wide. Doctrine: test verdicts should read the same under any machine load; a threshold over wall time relocates the flakiness to the threshold.

Evidence:

         4	//! Every test drives the policy stream by hand — a `futures` mpsc channel
         5	//! whose receiver is the `when` stream — so initiation timing is fully
         6	//! deterministic with no timers anywhere. The suite pins the driver's whole

       180	    let echo = futures::future::join(a_sessions.next(), b_sessions.next());
       181	    assert!(
       182	        timeout(Duration::from_millis(100), echo).await.is_err(),
       183	        "an echo session ran on a converged connection"
       184	    );

Resolution: Replace each negative window with `assert!(matches!(run_to_quiescence(futures::future::join(a_sessions.next(), b_sessions.next())), Err(Quiescence::Stalled)), "...")` (rumors::testing::Quiescence is public), moving the affected tests onto `common::wire::block_on` if calling the poller inside a runtime reads awkwardly; keep the 10 s DEADLINE only on positive waits, which fail loudly on expiry. Rewrite lines 4-6 to say what is true: cues are hand-fed and negative checks are stall-detected, with wall-clock deadlines remaining only as watchdogs on positive waits. Acceptance: no `from_millis(100)` remains in the file; each former negative names `Quiescence::Stalled`; the module doc no longer claims "no timers anywhere".
Construction: In a scratch copy make `Trigger::Tick(Some(Gossip::WhenChanged))` in src/peer/gossip.rs always initiate and insert `std::thread::sleep(Duration::from_millis(150))` in the driver's first poll; suppression_swallows_echoes_not_news passes at line 182 today, while the quiescence form fails on the first run.

### tests-disruption-handshake-15: Two driver terminal arms (when-exhaustion with staged preamble bytes; drop with staged bytes) have no test anywhere
- Where: tests/gossip_when.rs:438-442 (related: src/peer/gossip.rs:990-997, 1355-1373; src/tree/mirror/handshake.rs:265-280; src/peer/gossip/tests.rs:196-200)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep for `staged`, `Tick(None)`, `poison`, `drop(` in src/peer/gossip/tests.rs: the only `Staged` use is the bootstrap helper at 196-200; grep -rln 'gossip_when(' tests src: tests/gossip_when.rs, tests/session_stats.rs, src/rumors.rs, src/tutorial.rs, all over 8 KiB or 64 KiB links where a 30-byte write lands whole; read the two arms)
- Seen by: blind-spots [23]; refutation: confirmed (Staged::fill reads into the 30-byte buffer with one `read`, so no existing link capacity can leave a partial fill); history: no rationale found (bc9341c1 mentions Drive::drop "keeps covering the dropped-driver case" without citing a test)
- Owner-gated: no

when_exhaustion_then_hangup_both_end_cleanly covers `Trigger::Tick(None)` only with an empty staging buffer. The sibling arm at gossip.rs:994-997 (the `when` stream ends while a remote preamble is partially staged: serve that session, then end) and Drive's Drop at 1367-1373 (poison the link when dropped with staged bytes) exist for the partially-consumed-preamble state, which no test constructs; every gossip_when test in the tree runs over a link wide enough for the preamble to arrive in one read. A regression in either arm (returning None and stranding the peer mid-preamble; leaving the link unpoisoned so the next session misreads the remainder) passes the suite. The driver's own doc calls it "the case only this drop can see"; a branch whose justifying state no test reaches is unpinned, and the construction is cheap and deterministic.

Evidence:

       438	/// When the `when` stream ends with nothing in flight, the driver ends
       439	/// cleanly — and its still-running counterparty sees the dropped connection
       440	/// as a clean goodbye (end-of-stream at a session boundary), not an error.
       441	#[tokio::test(flavor = "current_thread")]
       442	async fn when_exhaustion_then_hangup_both_end_cleanly() {

    src/peer/gossip.rs
       993	                        Trigger::Tick(None) if drive.staged.is_empty() => return None,
       994	                        Trigger::Tick(None) => {
       995	                            drive.done = true;
       996	                            Led::Remote
       997	                        }
       ...
      1367	impl<T, B: BookmarkError, S> Drop for Drive<'_, T, B, S> {
      1368	    fn drop(&mut self) {
      1369	        if !self.staged.is_empty() {
      1370	            self.state.poison();
      1371	        }
      1372	    }
      1373	}

Resolution: Over `rumors::link::memory_with_capacity(1)`, let B run one-shot `gossip` and poll it a few times so its preamble crosses byte by byte; poll A's driver (`a_sessions.next().now_or_never()`) so `staged` holds between 1 and 29 bytes (assert this from the fixture, e.g. via a metered link). Then (a) drop A's cue sender and drive both under `run_to_quiescence`: A yields `Ok(Gossiped { led: Led::Remote, .. })` then None, B gets Ok; (b) in a second test drop A's driver instead and assert `run_to_quiescence(a.gossip(&mut a_link))` is `Ok(Err(Error::LinkPoisoned))`, while the existing empty-boundary drop leaves the link reusable. Acceptance: two new tests whose fixture asserts the staged byte count is strictly between 0 and 30; mutating gossip.rs:994-997 to `return None` and 1369-1371 to a no-op each fails exactly one of them.
Construction: As in the resolution; the capacity-one link is the only shape that can leave a partial fill, since `Staged::fill` reads with one `read` per poll.

### tests-disruption-handshake-16: Driver cancellation is sampled at one poll count; the family over the drop point is well-formed and absent
- Where: tests/gossip_when.rs:490-494 (related: tests/gossip_when.rs:468-534; tests/lifecycle.rs:40, 60-92)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read; lifecycle.rs uses a fixed MID_FLIGHT_POLLS = 4; grep for drop-point proptests in tests/ and src/**/tests.rs finds none)
- Seen by: blind-spots [31]; refutation: confirmed; history: no rationale found
- Owner-gated: no

The drop lands after exactly two `now_or_never` polls per side, early in the descent; the one-shot sibling in lifecycle.rs is likewise one point. The drop points that separate the documented post-commit exception (local commit done, epilogue not yet exchanged) from the general case are never reached. Every invariant the test asserts (each side keeps its own send, nothing beyond the union, both links poisoned once `begin` ran, a fresh connection converges) holds at every poll count, so the claim is a family and doctrine asks for a property test.

Evidence:

       490	        // Freeze the session mid-flight: a couple of single polls per side
       491	        // get the preambles (and the first protocol frames) onto the wire,
       492	        // well short of completion.
       493	        a_tx.unbounded_send(()).expect("driver alive");
       494	        for _ in 0..2 {

Resolution: Add a proptest `drop_after in 0usize..N` (N from a metered clean run's poll count) that polls both drivers `drop_after` times, drops them, and asserts the union and atomicity invariants plus poison-then-recover; record whether `a.snapshot().len() == 2` at the drop and, when it is, assert the drop landed after the last data frame. Acceptance: a proptest over the drop point exists in tests/gossip_when.rs (or tests/lifecycle.rs) and passes; any shrunk failure persists to proptest-regressions/.
Construction: Reuse the body of dropping_a_driver_mid_session_commits_nothing with the loop bound drawn from the strategy.

### tests-disruption-handshake-17: The severed-connection cut range `0..400` is unpinned, and with write-only cuts the certification clause's asymmetric case is unreachable
- Where: tests/gossip_when.rs:703-706 (related: tests/gossip_when.rs:695-701, 713-720, 757-770; src/peer/gossip.rs:1285-1319; tests/lifecycle.rs:155-175; tests/disruption.rs:422-448)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read; the reachability argument is reasoned from the epilogue's `try_join` and the test's own "failing side's drop surfaces as EOF" comment, not constructed)
- Seen by: blind-spots [25], api-economics [50]; refutation: reframed (folded in a new observation: the epilogue's try_join makes one-Ok-one-Err unreachable from write-only cuts); history: no rationale found (literal original to e43edcd7 under a different preamble; never re-derived)
- Owner-gated: no

The proptest draws both write cuts uniformly in `0..400` and nothing measures the fixture session's extent, so no check notices the family drifting to mostly-clean or mostly-past-the-end as the wire changes; disruption.rs shows the idiom for the same kind of constant (a metered envelope pinned two-sided). Separately, the doc promises the epilogue's certification "under arbitrary cut geometry", but with write-only cuts the non-trivial case (Ok on one driver, Err on the other) cannot arise: a cut on either side's marker write fails that side, its link drops, and the other side's marker read sees EOF, so both fail; the branch at 763-770 therefore runs only when neither cut landed inside the session. The asymmetric case has a deterministic witness in tests/lifecycle.rs (a read cut one byte short of the marker), so the property is protected, but this proptest's claim is stronger than what it can exercise.

Evidence:

       699	    /// already converged before any recovery (the epilogue's certification,
       700	    /// held under arbitrary cut geometry rather than only at pinned byte
       701	    /// boundaries), and a fresh clean connection converges the pair fully.

       703	    fn severed_connections_fail_loudly_and_recover(
       704	        a_write_cut in 0usize..400,
       705	        b_write_cut in 0usize..400,
       706	    ) {

Resolution: Name the range as a constant and pin it against a metered clean run of the same `pair()` fixture from both sides (as max_cut_spans_the_envelope_session does), or draw the cut as a fraction of the metered extent; add read cuts to the family (`read_cut: Some(..)` on either side) so the certification branch's asymmetric case is reachable, and let the doc claim only what the family can produce. Acceptance: a named constant with a committed two-sided pin replaces the literal 400; a generated or deterministic case yields Ok on one driver and Err on the other and passes the certification assertions.
Construction: Run `pair()` with both sends through `fault::metered` on a clean link and print each endpoint's written bytes; then add `b_read_cut in 0usize..RANGE` and observe the asymmetric branch firing.

### tests-disruption-handshake-18: The chaos interleaving proptest has no redaction arm
- Where: tests/gossip_when.rs:788-797 (related: tests/gossip_when.rs:809-888, 373-436)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the Op enum, op_strategy, and the feeder loop 843-859: no redact call exists in the proptest)
- Seen by: blind-spots [30]; refutation: confirmed; history: no rationale found
- Owner-gated: no

`Op` is sends, ticks, and pumps, so the property "under any interleaving of commits, ticks, and scheduler progress ... no session errors and full convergence" is established for inserts alone, while the doc's "commits" reads as including redactions. Redaction through the driver (a frontier-only advance, suppression keyed on `latest()`, a ceiling advance racing an in-flight session) is pinned only by the deterministic chain test at 373-436, one point in the space this generator should sweep.

Evidence:

       788	#[derive(Debug, Clone, Copy)]
       789	enum Op {
       790	    SendA,
       791	    SendB,
       792	    TickA,
       793	    TickB,

       810	    /// Chaos: under *any* interleaving of commits, ticks, and scheduler
       811	    /// progress on both sides of one connection, no session errors and
       812	    /// full convergence.

Resolution: Add `RedactA` and `RedactB` arms that redact the peer's own latest live message when one exists and count executed redactions; assert at the end `len == sends - executed_redactions` beside the existing hash and latest equalities. Acceptance: `op_strategy()` produces redaction ops; the final assertion accounts for them; the committed seed in proptest-regressions/gossip_when.txt still replays.
Construction: Extend the feeder's match with the two arms and re-run the proptest.

### tests-disruption-handshake-19: The preamble width is a bare `30` beside a named constant in a sibling suite
- Where: tests/gossip_when.rs:918-921 (related: tests/handshake.rs:24-27)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read both sites; both suites include tests/common)
- Seen by: api-economics [49]; refutation: confirmed; history: no rationale found (written the same day PREAMBLE_LEN was defined)
- Owner-gated: no

handshake.rs names the quantity `PREAMBLE_LEN` with its byte-by-byte derivation; this suite transcribes it as a literal. Named constants over magic numbers.

Evidence:

       918	        Err(Error::PreambleTruncated { received, expected }) => {
       919	            assert_eq!(received, 4, "the four delivered bytes are counted");
       920	            assert_eq!(expected, 30, "the V2 dialect width is named");

Resolution: Move `PREAMBLE_LEN` and its derivation comment to tests/common and import it in both suites. Acceptance: one definition of the preamble width under tests/.

### tests-disruption-handshake-20: The handshake suite's first line cites a module path that does not exist
- Where: tests/handshake.rs:1-1 (related: src/tree/mirror/handshake.rs:298; src/tree/mirror/streaming.rs:53)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep -rn 'remote::preamble|mod remote|fn preamble' src tests: `pub mod remote` only at src/tree/mirror/streaming.rs:53, the proxy; the function at src/tree/mirror/handshake.rs:298)
- Seen by: structure-prose [8], blind-spots [34], api-economics [45]; refutation: confirmed (a true ghost of moved code: remote/preamble.rs existed at 2b6618ae and was merged into the shared handshake at f6cf2579); history: contradicts the AGENTS.md hard rule
- Owner-gated: no

The preamble exchange is `tree::mirror::handshake::preamble`; nothing named `mirror::remote::preamble` exists, and `remote` is the streaming proxy. A first sentence stands alone in a listing and here points the maintainer at the wrong module.

Evidence:

         1	//! Protocol preamble exchange (`mirror::remote::preamble`).

Resolution: Cite `tree::mirror::handshake::preamble`, or drop the parenthetical (the next sentence already says what is driven). Acceptance: the cited path resolves under src/.

### tests-disruption-handshake-21: protocol_constants_match_spec compares a test constant to its own definition
- Where: tests/handshake.rs:57-61 (related: tests/handshake.rs:29-32, 54-56)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (git show 368da2a5 -- tests/handshake.rs removed the `PROTOCOL_MAGIC` and V1 half; lines 30-32 define V2_OPENING with the same three leading bytes)
- Seen by: structure-prose [6], api-economics [39]; refutation: confirmed (line 59 is a real pin of the public discriminant and stays); history: deliberate but expired (at birth the test compared crate constants to literals; 368da2a5 deleted the live half and left the tautology)
- Owner-gated: no

The second assertion checks `V2_OPENING[..3]` against the bytes that V2_OPENING's own definition spells two lines above; it can only fail if the file disagrees with itself. The doc says the opening "starts every preamble", but no preamble is observed. A check whose only input is itself catches nothing.

Evidence:

        54	/// The fixed markers match the hand-encoded layout: the self-described
        55	/// CBOR opening starts every preamble, and the wire version is the
        56	/// dialect's discriminant.
        57	#[test]
        58	fn protocol_constants_match_spec() {
        59	    assert_eq!(Protocol::V2 as u16, 2);
        60	    assert_eq!(&V2_OPENING[..3], &[0xd9, 0xd9, 0xf7]);
        61	}

Resolution: Delete line 60; keep the discriminant pin, or fold `Protocol::V2 as u8` into the outgoing-preamble check proposed in finding 22 (byte 11 of the bytes alice writes), which pins the discriminant against the wire rather than against the enum. Acceptance: no assertion in tests/handshake.rs compares a local constant to a literal restating it.

### tests-disruption-handshake-22: The handshake suite discards alice's preamble at every fake-peer site, repeats a twelve-line scaffold six times, and never checks the local set
- Where: tests/handshake.rs:94-97 (related: tests/handshake.rs:5-6, 10-12, 125-126, 161-162, 190-191, 225-226, 258-259; src/tree/mirror/handshake.rs:98-107; src/network.rs:80; src/tree/mirror/handshake/tests.rs:244-246, 268-281)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (grep for `got` in tests/handshake.rs: filled by read_exact at six sites and never read afterwards; grep BootstrapRetireConflict tests/: no hits; src/network.rs:80 shows `to_bytes` is pub(crate))
- Seen by: structure-prose [5], blind-spots [29], api-economics [39]; refutation: confirmed (the rejection variants pin layout positions indirectly, so it is half an oracle, not none); history: no rationale found (`got` never asserted since 8b1ade01; the "independent oracle" sentence was added at 4dd2053c over an inbound-only suite)
- Owner-gated: no

The module doc calls the suite "an independent oracle of the documented wire spelling", but every fake peer reads alice's 30 bytes into `got` and drops them: the encoder's spelling is never compared to the hand-transcribed layout, and the decoder is exercised only through rejections. The doc also promises errors surface "rather than corrupting the local rumor set", and no test inspects alice's set after an error. The remaining preamble diagnosis, `Error::BootstrapRetireConflict`, is unit-tested but has no end-to-end mapping here. Six copies of the same scaffold (84-110, 115-147, 153-177, 183-210, 218-245, 251-276) differ only in the reply bytes and the expected variant. The written preamble is pinned elsewhere (the unit test prefix_matches_the_writers and the insta snapshots), so the wire is not unprotected; this suite does not do what it says it does.

Evidence:

        10	//! The layout is transcribed here by hand, deliberately: this suite is an
        11	//! independent oracle of the documented wire spelling, so it must not
        12	//! derive the bytes from the code under test.

        94	        let mut got = [0u8; PREAMBLE_LEN];
        95	        b_r.read_exact(&mut got).await.expect("fake peer read");
        96	        let reply = preamble(bad_opening, Protocol::V2 as u8, INTENT_REMAIN);
        97	        b_w.write_all(&reply).await.expect("fake peer write");

Resolution: Add one helper `async fn alice_against(reply: &[u8]) -> (Result<Gossiped, Error>, Rumors<String>)` that builds alice, runs the fake peer, asserts `got[..13] == preamble(V2_OPENING, Protocol::V2 as u8, INTENT_REMAIN)[..13]` and `got[29] == INTENT_REMAIN` (bytes 13..29 are the universe's network id, which has no public accessor; skip them), and drops the write half after writing (the truncation case passes `&partial[..6]`). Each rejection test becomes: build the reply, call the helper, match the variant, assert `alice.snapshot().is_empty()`. Add a `bootstrap_retire_conflict_surfaces_error` case (network `[0; 16]`, intent 1). Acceptance: one helper and six tests of a few lines each; changing `Protocol::V2 as u64` to `3` in `Preamble::encode` (src/tree/mirror/handshake.rs:102) fails every rejection test, not only the snapshot suite; every error test asserts the local set is unchanged; `Error::BootstrapRetireConflict` appears in tests/handshake.rs.
Construction: In a scratch copy set the encoded version to 3 at handshake.rs:102; today only handshake_roundtrip_succeeds fails (both sides real) and no rejection test observes the outgoing byte.

### tests-disruption-handshake-23: handshake_precedes_protocol_traffic re-runs the magic-mismatch case under a doc it does not check
- Where: tests/handshake.rs:247-262 (related: tests/handshake.rs:81-110; src/tree/mirror/handshake.rs:111-118)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read both bodies; both match `Error::MagicMismatch` from `Preamble::decode`'s prefix check)
- Seen by: structure-prose [7]; refutation: confirmed; history: deliberate but expired (at 70e28462 the preamble rode a length-prefixed frame and the reply declared a 64-byte payload, so the doc's length clause described the body; 4dd2053c made the preamble a self-described 30-byte item and the clause lost its referent)
- Owner-gated: no

The body writes 30 'X' bytes and expects MagicMismatch: mechanically the same as magic_mismatch_surfaces_error with different bytes. The doc claims rejection happens "before any peer-declared protocol frame length can be read or trusted", which nothing in the body observes; only the variant is matched. Two tests for one behavior with different stories is drift waiting to happen, and a testdoc states what the body checks.

Evidence:

       247	/// The preamble must be the connection's first bytes: a peer that skips it and
       248	/// goes straight to protocol traffic is rejected as a magic mismatch before
       249	/// any peer-declared protocol frame length can be read or trusted.
       ...
       262	        let reply = [b'X'; PREAMBLE_LEN];

Resolution: Fold into magic_mismatch_surfaces_error as a second reply pattern (a small table of openings), or rewrite the doc to claim only what is asserted: any 30 bytes without the rumors opening are diagnosed as MagicMismatch quoting the first six. Acceptance: each handshake.rs test's doc names a distinct observed behavior.

### tests-disruption-handshake-24: Ragged doc paragraphs: a short first line followed by full-width lines
- Where: tests/handshake_liveness.rs:56-59 (related: tests/handshake_liveness.rs:41-44, 81-82, 290-295, 316-318; tests/gossip_when.rs:153-157, 359-364, 472-475, 540-542, 784-787; tests/disruption.rs:80-83, 575-580; tests/hop_trace.rs:632-634)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (each cited site read)
- Seen by: structure-prose [17]; refutation: confirmed; history: the splits are deliberate (dfd19c44 inserted a blank `///` after each first sentence for doclint's summary rule); the raggedness is the unreflowed remainder
- Owner-gated: no

Several doc comments open a paragraph with a fragment of a few words and continue at full width, the trace of the first-sentence split applied without reflowing. rustfmt does not reflow comments, so this stays until someone does it by hand. Any reflow must keep the blank `///` line so the summary stays one sentence.

Evidence:

        56	/// ITC versions are
        57	/// bit-packed and stay compact even across many parties, so the one-byte
        58	/// window is what guarantees a multi-fill greeting; this floor guards the
        59	/// fixture against normalizing back to a trivial frame.

Resolution: Reflow the listed paragraphs, keeping the blank `///` separator. Acceptance: no doc paragraph in the partition has a first line under 40 characters followed by a line over 60.

### tests-disruption-handshake-25: The fixture self-check comment states a purpose the GREETING_FLOOR doc contradicts
- Where: tests/handshake_liveness.rs:132-134 (related: tests/handshake_liveness.rs:30-32, 52-60)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read 30-32, 52-60, 132-140)
- Seen by: blind-spots [33]; refutation: confirmed; history: no rationale found (both statements born together at b37c2d45, never reconciled)
- Owner-gated: no

At MIN_CAPACITY = 1 every frame of two or more bytes overflows the window, so the version-width check guards the fixture's branchiness, not whether the matrix exercises the hazard class. The constant's own doc says exactly that ("the one-byte window is what guarantees a multi-fill greeting; this floor guards the fixture against normalizing back to a trivial frame"); the inline comment says the opposite.

Evidence:

       132	    // Fixture self-check: the greeting's version frame must dwarf the
       133	    // minimal window, or the matrix stops exercising the hazard class. A
       134	    // width bound only — cells never assert greeting contents.

Resolution: Reword the inline comment to the const doc's claim: the floor keeps the seasoned version wide and branchy so cells exercise wide-version wire shapes; the one-byte window alone guarantees multi-fill. Acceptance: the inline comment and the GREETING_FLOOR doc state the same purpose.

### tests-disruption-handshake-26: No default-window peer runs a descent over the one-byte link
- Where: tests/handshake_liveness.rs:143-148 (related: tests/handshake_liveness.rs:7-12, 154-165, 255-258, 335-338; tests/common/wire.rs:209-218; src/tree/mirror/streaming/window.rs:4-12; tests/disruption.rs:706, 916, 936)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep for `memory_with_capacity(1\b|MIN_CAPACITY` across tests and src: only this file, window_corners.rs:158 with budget 0, which `Window::from_budget` floors to capacity one everywhere, and the link conformance test; every descent cell here uses `seasoned()` or `bootstrap_fork_async`, both floor-pinned)
- Seen by: blind-spots [22]; refutation: reframed (the design docs argue liveness at any width: window.rs:11-12 "capacity only relaxes the wait graph, so every schedule live at the floor stays live at any width", so this is a coverage gap on a documented argument, not an uncertified configuration; severity lowered to low); history: partial rationale (db2718d4 pinned the floor blanket-wide; 919132dc swept the window dimension through the intra-process engines only, with no note deferring this matrix or the TCP leg)
- Owner-gated: no

Every descent cell in the one-byte matrix is floor-versus-floor: `seasoned()` pins `sync_window_floor()` and `bootstrap_fork_async` pins `WindowChoice::Floor`. The only default-window peers over MIN_CAPACITY are bootstrap newcomers with empty trees (255-258, 335-338), whose descent is one-sided supply. The documented claim that wider windows inherit the floor's liveness is therefore exercised only over 8 KiB links (the sim's window sweep), never against the transport that hides nothing, and the inter-process TCP engine likewise pins the floor at 706, 916, 936 while its intra-process sibling sweeps WindowAssignment. Making the argument's generalization observable is cheap: the harness already has `WindowChoice::apply`.

Evidence:

       143	/// A seasoned replica: a fresh universe with a wide version.
       144	async fn seasoned() -> Rumors<u64> {
       145	    let seed: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
       146	    season(&seed, 0).await;
       147	    seed
       148	}

Resolution: Parameterize `seasoned()` and `seasoned_pair()` over WindowChoice and instantiate the descent cells (divergent, bulk_initiator, retire, empty_meets_populated) at least at Default-vs-Default and one asymmetric Floor-vs-Default; in disruption.rs let the inter-process parent peers and the child take a window choice from the plan. Acceptance: new cells such as `divergent_default_window` and `divergent_asymmetric_window` exist under `block_on` and pass; `ProcPlan` carries a window field drawn from `arb_window_choice`.
Construction: Copy divergent_session with `WindowChoice::Default.apply(...)` on both seeds and run under `block_on`; a wedge surfaces as a deterministic Stalled failure.

### tests-disruption-handshake-27: The shape/cell split, the "matrix" framing, and the "V2" qualifiers are residue of the retired protocol column
- Where: tests/handshake_liveness.rs:370-375 (related: tests/handshake_liveness.rs:1, 167-171, 173-178, 368-431)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (git show 368da2a5 -- tests/handshake_liveness.rs: every shape lost its `protocol` parameter and the section comment changed from "The `_v1`/`_v2` cells below instantiate each shape per protocol" to "The test cells below instantiate one shape each"; grep -c V2 is 9, one per cell doc)
- Seen by: structure-prose [9]; refutation: confirmed; history: deliberate but expired (b37c2d45 built shapes x protocols; 368da2a5 removed the column and kept the indirection)
- Owner-gated: no

Nine async shape functions each have a one-line `#[test]` wrapper with a second doc comment restating the shape's doc. Before 368da2a5 the wrappers were the `_v1`/`_v2` cells of a shapes-by-protocol grid, which the "matrix" word (line 1 and the header at 368) and the "V2" in every cell doc served. With one dialect the qualifier distinguishes nothing and the two-layer structure costs nine pairs of docs that must agree. Machinery outlives the constraint that justified it.

Evidence:

       370	/// A V2 session between converged seasoned replicas stays live and
       371	/// re-converges over a one-byte-window link.
       372	#[test]
       373	fn converged() {
       374	    block_on(converged_session());
       375	}

Resolution: Inline each shape body into its `#[test] fn` inside `block_on(async { ... })`, keep one merged doc comment per test, replace "matrix" with the list it is, and drop "V2" from the cell docs (the dialect's name belongs where wire bytes are spelled). Acceptance: one function and one doc comment per session shape; `grep -c V2 tests/handshake_liveness.rs` is 0.

### tests-disruption-handshake-28: The `#[path]` inclusion of benches/support/latency.rs generates dead_code allows at every include site and inside the module
- Where: tests/hop_trace.rs:26-29 (related: tests/gossip_pipelining.rs:11-15; tests/window_corners.rs:15; tests/window_knee.rs:20; tests/window_operator.rs:25; tests/tradeoff_probe.rs:31; tests/latency_link.rs:14; benches/support/latency.rs:398-400, 463-466, 511-514, 534-537)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep -rn 'benches/support/latency.rs' tests benches: seven includers, each under `#[allow(dead_code)]`; the four per-item allows in latency.rs each carry the comment "the module is `#[path]`-included by several targets")
- Seen by: structure-prose [14]; refutation: confirmed; history: no rationale found (`#[path]` is Cargo's conventional idiom for sharing bench support with tests, which is why this is owner-gated)
- Owner-gated: yes: restructuring the support code into a dev-only crate or a feature-gated module is a build-surface decision

Seven test binaries compile latency.rs by `#[path]`, each needing `#[allow(dead_code)]`, and latency.rs carries four per-item allows each justified only by the inclusion mechanism. Infrastructure is suspect when it generates its own maintenance cascade; each allow exists because the support code sits in the wrong compilation unit.

Evidence:

        26	// Only the pipe layer is reused; the wire driver here is trace-aware.
        27	#[allow(dead_code)]
        28	#[path = "../benches/support/latency.rs"]
        29	mod latency;

Resolution: Move benches/support into a dev-only path crate under crates/ (or a `test-internals`-gated module of rumors) so it compiles once and unused items are simply unused; delete the allows and their comments. Acceptance: `grep -rn 'path = "../benches/support' tests` returns nothing; latency.rs has no `#[allow(dead_code)]`.

### tests-disruption-handshake-29: The hop_trace import block bleeds into the first item, and the tracer keys pipes by primitive sentinels
- Where: tests/hop_trace.rs:49-54 (related: tests/hop_trace.rs:63-69, 72-77; tests/common/wire.rs:18-22; tests/common/action.rs:7-9)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the sites)
- Seen by: structure-prose [20]; refutation: confirmed; history: no rationale found (the serde placement is an artifact of c6fe4018's mechanized import sweep; the primitive ids are from 7d8891b2)
- Owner-gated: no

The two serde imports sit after `use latency::...` and run straight into the DELAY doc comment with no blank line, which rustfmt will not insert; tests/common/wire.rs:20-22 and action.rs:7-9 show the same bleed. Separately, PipeId keys pipes by `side: char` ('A'/'B'), direction by `write: bool`, and the control stream by a `CONTROL: u8 = 0xFF` sentinel in the serial field, where two-variant enums and `enum Serial { Control, Data(u8) }` are the types-first shape.

Evidence:

        49	use latency::{DelayedReader, DelayedWriter, delayed_pipe};
        50	
        51	use serde::Serialize;
        52	use serde::de::DeserializeOwned;
        53	/// One-way link delay; whole milliseconds per the timer wheel's grain.
        54	const DELAY: Duration = Duration::from_millis(10);

        64	struct PipeId {
        65	    side: char,
        66	    serial: u8,
        67	}
        68	
        69	const CONTROL: u8 = 0xFF;

Resolution: Fold the serde imports into the sorted block and add the blank line (here and at the two tests/common sites); optionally replace `side: char`, `write: bool`, and the CONTROL sentinel with small local enums. Acceptance: one import block followed by a blank line; no 0xFF sentinel in PipeId.

### tests-disruption-handshake-30: Trace::hops reimplements latency::hops_on_lattice, and first_write_hop divides without the lattice check
- Where: tests/hop_trace.rs:137-143 (related: tests/hop_trace.rs:125-128, 165; benches/support/latency.rs:538-553)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both; latency.rs's reader is nanosecond-strict and carries `#[allow(dead_code)]` for exactly this include)
- Seen by: structure-prose [10], api-economics [46]; refutation: confirmed; history: deliberate but expired (hop_trace's local reader predates hops_on_lattice, added at 814f07ad without retargeting hop_trace)
- Owner-gated: no

hop_trace `#[path]`-includes latency.rs, whose `hops_on_lattice(elapsed, delay)` performs this division with the exactness assert; Trace::hops re-derives it in milliseconds, and first_write_hop divides with no assert at all, contradicting the doc's "exact, not a rounding". One reading for one quantity; the two readers have already drifted in strictness.

Evidence:

       137	        let millis = last.as_millis() as u64;
       138	        assert_eq!(
       139	            millis % DELAY.as_millis() as u64,
       140	            0,
       141	            "every traced event lands on an exact delay multiple"
       142	        );
       143	        millis / DELAY.as_millis() as u64

       165	        first.as_millis() as u64 / DELAY.as_millis() as u64

Resolution: Return `latency::hops_on_lattice(last, DELAY)` from hops() and `latency::hops_on_lattice(first, DELAY)` from first_write_hop(); drop the local assert and adapt the return type (u32). Acceptance: no `as_millis() ... / DELAY.as_millis()` arithmetic remains in tests/hop_trace.rs outside report()'s display code.

### tests-disruption-handshake-31: hop_trace's insertion fixture is byte-identical to gossip_pipelining's under a doc that says "reduced scale"; send_random has seven copies; bootstrap_fork and no-op budget calls are duplicated
- Where: tests/hop_trace.rs:437-446 (related: tests/gossip_pipelining.rs:23-28, 64-94; tests/hop_trace.rs:460-462, 475-493, 546-548, 637-639; benches/support/wire.rs:39-56; tests/window_census.rs:73; tests/window_corners.rs:63; tests/window_knee.rs:214; tests/window_operator.rs:82; benches/window_wallclock.rs:89; src/tree/mirror/streaming/window.rs:547-555)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (grep for `fn send_random` and the seed 0x9e37_79b9_7f4a_7c15; read window.rs:547-555 for `WindowConfig::default() == Budget(DEFAULT_SYNC_MEMORY_BUDGET)`; read benches/support/wire.rs:39-56)
- Seen by: structure-prose [3], api-economics [40]; refutation: confirmed (gossip_pipelining's `.sync_memory_budget` call is that test's subject and stays); history: "reduced scale" was false from birth relative to the pipelining test (7d8891b2's message compares to the bench fixture); the explicit budget calls are the pre-db2718d4 escape that commit removed from benches but not from this test
- Owner-gated: no

diverged_insertions builds the same pair as gossip_pipelining's diverged_pair (COMMON 2_048, DIVERGENT_PER_SIDE 512, seed 0x9e37_79b9_7f4a_7c15, default window), yet its doc says it mirrors the pipelining shape "at reduced scale": a doc claim the constants contradict. send_random has seven identical copies across the standalone measurement binaries and one bench; hop_trace's bootstrap_fork duplicates benches/support/wire.rs::bootstrap_fork except for a capacity and a `.sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)` call that is the default already. These binaries avoid `mod common;` (its `#![allow(dead_code)]` pulls in the whole sim engine) but all `#[path]`-include benches/support, which is the natural home.

Evidence:

       437	/// Two production-window V2 peers with a shared prefix and divergence on
       438	/// both sides, mirroring the pipelining test's shape at reduced scale.
       439	fn diverged_insertions() -> (Rumors<u64>, Rumors<u64>) {
       440	    const COMMON: usize = 2_048;
       441	    const DIVERGENT_PER_SIDE: usize = 512;
       442	
       443	    let left = Peer::seed()
       444	        .sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)
       445	        .into_rumors();
       446	    let mut rng = SmallRng::seed_from_u64(0x9e37_79b9_7f4a_7c15);

    tests/gossip_pipelining.rs
        24	const COMMON: usize = 2_048;
        28	const DIVERGENT_PER_SIDE: usize = 512;
        69	    let mut rng = SmallRng::seed_from_u64(0x9e37_79b9_7f4a_7c15);

Resolution: Fix the doc ("the same shape as the pipelining test"). Move send_random, bootstrap_fork, and a `diverged_pair(common, per_side, seed)` fixture into a benches/support module and include it from gossip_pipelining, hop_trace, window_census, window_corners, window_knee, window_operator, tradeoff_probe, and benches/window_wallclock. Drop the no-op `.sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)` in hop_trace (443-445, 460-462, 490, 546-548, 637-639); keep it in gossip_pipelining, where the public knob is the test's point. Acceptance: `grep -rn 'fn send_random' tests benches` returns one definition; one copy of the 0x9e37_79b9_7f4a_7c15 fixture exists; hop_trace's fixture docs make no scale comparison the constants contradict.

### tests-disruption-handshake-32: transfer_pair's comments misattribute its determinism and overstate what its self-checks catch
- Where: tests/hop_trace.rs:529-530 (related: tests/hop_trace.rs:535, 546-548, 585-600; src/peer.rs:213-215; src/tree/typed/hash.rs:242-258)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read peer.rs:213-215: `seed_rng` draws only the Network; read hash.rs: a leaf's path is Sha3_256 of the version's canonical bytes; read the self-checks 587-600, which use the test-local `path_radix`)
- Seen by: structure-prose [21], api-economics [47] (the seed_rng half); refutation: 21 confirmed, 47 reframed (the seed_rng half holds; the `sync_window_floor` half is the documented suite convention and is dropped); history: 21 deliberate but expired (the self-check read the crate's own key bytes at 55d76d5c; ba8045c5 switched to a test-local radix and kept the sentence), 47 no rationale found
- Owner-gated: no

The RNG passed to `Peer::seed_rng` picks only the Network id; versions come from `Party::seed()` ticks and paths are hashes of version bytes, so the seed is inert for the fixture shape and the comment's "deterministic function of the seeded universe" misattributes the determinism (the sibling fixtures use `Peer::seed()`). The self-checks use the test's own `path_radix`, so a change to the crate's leaf hash would leave them self-consistent while the session shape, and hence the hop pin at 621, changed; only version-assignment drift fails at the self-checks, the opposite of what the comment says.

Evidence:

       529	    // Stage by pool search: paths are version-derived, so the shape is a
       530	    // deterministic function of the seeded universe and send order — send

       546	    let left = Peer::seed_rng(&mut SmallRng::seed_from_u64(0))

       585	    // Fixture self-checks: mirror the required shape so drift in hashing
       586	    // or version assignment fails here, not in the hop arithmetic.

Resolution: Use `Peer::seed()` and reword 529-530 to "a deterministic function of the send order"; reword 585-586 to "so drift in version assignment fails here, not in the hop arithmetic; a change to the leaf hash shows up as a hop-count mismatch, since the fixture spells the documented SHA3-256 path itself". Acceptance: `seed_rng` is used only where the network id matters; both comments name only what the code does.

### tests-disruption-handshake-33: The gate's testdoc never sees `#[pollster::test]`: nine tests in this partition and 29 crate-wide are unchecked
- Where: tools/testdoc:20-23 (related: tools/testdoc:85-100; justfile:184-186; tests/handshake.rs:65, 83, 114, 152, 182, 217, 250; tests/gossip_when.rs:476, 680; tests/changes.rs)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (ran `python3 tools/testdoc` on a scratch probe under the final/ scratch directory containing undocumented `#[pollster::test]`, `#[test]`, and `#[tokio::test(flavor = "current_thread")]` functions: exit 1 naming only `undocumented_plain` and `undocumented_tokio`; the pollster case passed silently. `grep -rn 'pollster::test' tests src | wc -l` is 29: handshake.rs 7, gossip_when.rs 2, changes.rs 7, src 13)
- Seen by: api-economics [37]; refutation: confirmed (independently reproduced); history: deliberate but expired (the alternation was complete at 358c6b1a; `pollster::test` arrived two days later and the regex was never widened; the self-test pins only the tokio form)
- Owner-gated: no

TEST_ATTRIBUTE enumerates `test|tokio::test|async_std::test|rstest|test_case` anchored at `#[`, so `#[pollster::test]` never matches and those tests' doc comments are not held by the gate (justfile:184-186 runs `./tools/testdoc .`). All 29 currently carry docs, but nothing enforces it, and the self-test has no pollster case. AGENTS.md: "The gate's testdoc checks that the comment exists"; a check that exempts one attribute form is a board nothing enforces for that form.

Evidence:

        20	TEST_ATTRIBUTE = re.compile(
        21	    r"^\s*#\[\s*(?:test|tokio::test|async_std::test|rstest|test_case)"
        22	    r"\s*(?:\([^]]*\))?\s*\]\s*$"
        23	)

Resolution: Widen the alternation to accept any path ending in `test`, e.g. `(?:[A-Za-z_][A-Za-z0-9_]*::)*test` alongside `rstest|test_case`, and add a self-test case with an undocumented `#[pollster::test] async fn x() {}` expected to be reported. Acceptance: `./tools/testdoc --self-test` passes with the new case, and the tool exits 1 naming an undocumented `#[pollster::test]` function.

## Positives

- disruption.rs value_oracle_tripwires_catch_known_bad_mechanisms (159-219) and value_oracle_survives_committed_retire_chain (268-350) construct both known-bad mechanisms (a suppressed redaction, a dropped insert) and show them failing the checks in the same run that shows the uncorrupted ledger passing: the adequacy discipline done exactly right.
- disruption.rs max_cut_spans_the_envelope_session (352-448) derives MAX_CUT from a metered envelope session using the same counters the cuts spend, pins it from both sides, derives the envelope's value count from the generator's own exported bounds (ENVELOPE_VALUES_PER_SIDE), and states the dominance premise honestly (exact on value count, representative on version shape).
- disruption.rs custody_chain_loss_is_transitive (221-248) records the concrete reviewed counterexample as the test's invariant, so the transitive custody rule carries its motivation.
- handshake_liveness.rs runs every session shape under the closed-world quiescence poller, so a deadlock surfaces as a deterministic stalled-poll failure with the cell name attached; the seasoning fixture self-checks that the greeting version dwarfs the window (GREETING_FLOOR) without pinning greeting bytes; cells assert liveness and convergence only; and every successful session is held to the clean-drain invariant at its boundary.
- hop_trace.rs turns virtual time into an exact instrument (bucket k is hop k), documents the run command for reading the trace, and transfer_pair (528-602) asserts its staged shape (two leaves under one root radix, ballast elsewhere, smaller set initiates) before any hop arithmetic runs.
- handshake.rs transcribes the preamble layout by hand and says why (10-12): an oracle that must not derive its expectations from the code under test.
- gossip_when.rs dropping_next_futures_loses_nothing (574-614) states why it uses a minimal executor (it pins the driver's no-Tokio promise) and runs poll cancel-safety under the quiescence detector; a_driver_on_a_poisoned_link_fails_fast proves the fail-fast without a counterparty; severed_connections_fail_loudly_and_recover asserts the epilogue's certification whenever any Ok appears.
- common::wire::assert_control_drained is applied at every successful session boundary across the partition, turning latent control-stream desynchronization into an immediate failure at the session that caused it.
- Every `#[test]` in the partition carries a doc comment that states a behavior, and assert messages say what a failure means rather than restating the condition.

## Open questions for Finch

- gossip_pipelining's loose bound (`< 24` hops) beside hop_trace's exact pin (`== 7`) on the same fixture: HOP_BUDGET's doc and 814f07ad record it as a regime separator meant to survive a deliberate re-pin of the exact figure. Keep it as such, or dissolve the binary into hop_trace? Recommendation: keep the bound, share the fixture, and add the instrument equality and the measured floor leg (findings 10 and 31); if kept, say in the module doc that the bound is a regime check that does not move when the exact pin is re-accepted.
- `Bootstrap::join` returns `Result<Option<Peer>>` while `BookmarkedBootstrap::join` returns a typed `Joined`; the test corpus double-`expect`s at seven sites (gossip_pipelining.rs:79-81, hop_trace.rs:487-489, handshake_liveness.rs:255-257 and 335-337, disruption.rs:931-935, common/wire.rs:257-259, common/sim.rs:511-513). 55e34738 recorded the asymmetry as deliberate ("nothing can fail after the session"); the join doc states what `Ok(None)` means but not why the shape is an Option. Recommendation: state the type-shape rationale inline in `join`'s doc and leave the API; a reshaping toward a shared outcome vocabulary is an owner decision the seven sites do not by themselves justify.
- Is the OS-process boundary in disruption.rs load-bearing? The library holds no process-global state the children could share, and a dying peer's ConnectionRefused can be produced in-process by dropping a listener. Recommendation: state in the module doc what the boundary proves (real sockets, process death mid-session, a separately-initialized runtime); if nothing, the exec protocol could dissolve into an in-process TCP variant of run_plan.
- Should ChildPlan gain a redaction step so the TCP leg covers deletion honoring and the value ledger (finding 4)? Recommendation: yes; a child redacting one of its own sends before the final gossip, with the parent asserting the redaction's absence, is small and closes the leg's only structural blind spot.
- Can gossip_when's two proptests run under `run_to_quiescence` instead of the reused tokio runtime with 10 s watchdogs, making a wedge a deterministic Stalled rather than a 10 s wait? `Op::Pump` uses `tokio::task::yield_now`, which may need a runtime. Recommendation: try it; if yield_now works without a runtime the DEADLINE watchdogs on those tests can go too.
- Finding 7's severity: the rubric puts a harness bug that masks failures at high; the refutation pass argued medium because the masked class is confined to the TCP serving path. I kept high because that path is the engine's unique coverage. Recommendation: fix regardless; the severity only affects scheduling.

## Dropped

- [43] `Bootstrap::join` returns `Result<Option<Peer>>`: deliberate and recorded (55e34738); the seven double-expect sites are not new evidence; moved to open questions.
- [47] `sync_window_floor` on every alice in handshake.rs: refuted; pinning the floor is the documented suite convention (window.rs:549-553, wire.rs:209-212) and needs no comment at the sites. The seed_rng half survives in finding 32.
- [51] Heal loop never re-checks convergence after its final mesh round (tests/common/sim.rs:891-905; same shape at tests/common/peer.rs:155-160): out of this partition's file list; a false failure only, never a masked pass. Relay to the tests/common reviewer.
- [42] HOP_BUDGET's floor regime is argued, not constructed: merged into finding 10 with the refutation's correction that window_corners measures the floor (budget 0 floors every capacity at one) on a different fixture.
- [4], [40] Dissolve gossip_pipelining into hop_trace: the bound's coexistence with the exact pin is a recorded decision that holds; the actionable residue (shared fixture, instrument equality, measured floor) lives in findings 10 and 31 and the dissolve question is in open questions.
- [18]'s 'real TCP', 'real peer', 'real parallelism', 'real sessions' sites: refuted; each contrasts with a simulated or hand-driven counterpart and carries mechanism.
- Refutation new item 3 (a note at window_corners.rs that budget 0 is floor-equivalent): out of this partition's file list; worth a one-line note there or in `Peer::sync_memory_budget`'s docs.
- Duplicates merged: {0, 24, 38} into finding 14; {1, 41} into 9; {2, 44} into 4; {3, 40} into 31; {4, 28, 40, 42} into 10; {5, 29, 39} into 22; {6, 39} into 21; {8, 34, 45} into 20; {10, 46} into 30; {11, 12, refutation new 4} into 12; {15, 35} into 6; {19, 27} into 5; {21, 47} into 32; {25, 50, refutation new 1} into 17; {48, refutation new 2} into 11.
