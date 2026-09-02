# Partition streaming-tests: The streaming protocol test suites: fixtures, skeleton, local equivalence, faults, capacity, stats, wedge

## Partition summary

This partition is the in-process test suite for the streaming mirror, nine files under `src/tree/mirror/streaming/tests.rs` and `tests/`, 3075 lines, all test code (`#[cfg(test)] mod tests` under `streaming.rs`). `tests.rs` holds the shared harness: two `Local`-backed endpoints at `WindowConfig::FLOOR` driven through `mirror` under the closed-world poller `run_to_quiescence`, with optional channel and backend poll schedules, a progress trace validated on every run, and a payload-erased reply transcript; `Tree::join` is the differential oracle. `fixtures.rs` builds deterministic trees with hand-placed paths (one-sided pairs, prefix-cell pairs, pyramids, the full-depth comb, and the three-tree `Divergence` generator). On top sit six suites: `capacity.rs` pins channel-capacity boundaries and parent-delay stall probes; `faults.rs` injects reply violations, greeting lies, and backend failures through the connected driver in both orientations; `stats.rs` pins the session counters against a prefix-closure oracle; and `skeleton.rs`, `wedge.rs`, `local_eq.rs`, and `announced.rs` bridge real sessions to the Lean model (the `Mux.wedge` witness, the `viewEnc`/`LocalEq` projection, and the payload-independence reconstruction).

The engineering is strong where it is aimed. Every test carries a doc comment; the oracles are independent and derived rather than hand-tallied (join for content, prefix closure split by depth parity for disputes, the transcribed Lean literal for the wedge); the capacity witness is pinned from both sides of its boundary (stalls at 253, completes at 254) with liveness floors on every queue role; the skeleton decoder audits every publication against the model's count and parity laws as a total check; and determinism is used uniformly (no sleeps, runtimes, or spawned processes; thread-local instruments scoped by `with_*` guards). The Rust mirrors of the Lean definitions match the Lean source, and the deviations from it are recorded at the site.

The defects concentrate in three places. First, one harness weakness that masks failures: the stall probes in `capacity.rs` collapse completion, violation, and poll-budget exhaustion into one boolean, so every "must complete" assertion passes on a protocol error or a livelock. Second, verification gaps at the edges of what the suite claims: the whole-subtree shed count is never observed above one anywhere in the tree, the fault-injection step range excludes the leaf-height terminal phase on both sides (and a committed seed records a failure at exactly that step), and the join-oracle differential omits the deep-spine generator built for divergence below the root. Third, prose: two doc comments describe code that no longer exists, `wedge.rs` names seed paths the gate now forbids, and the bridge docs identify what they test by campaign roster tags (`B5`, `T3`, `F4`, `finding #7`, `Bridge N`) whose definitions live only in `.agent-notes/` and `formal/doc/`. The rest is legibility work on a test harness that grew by accretion: the session body is spelled nine times behind a six-function ladder, the fixture file hand-rolls what its own builders express, and one family-shaped test drives `TestRunner` by hand and so forfeits shrinking and seed persistence.

## Findings

### streaming-tests-1: The tests.rs module doc maps three of its eight submodules
- Where: src/tree/mirror/streaming/tests.rs:1-5 (related: src/tree/mirror/streaming/tests.rs:27-34)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); the history pass verified via `git log -L1,5` that the doc dates from ddc9a2d8, when the three named modules were the only submodules
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (complete when written; five modules added later in 97dfbcdf and 7626543e without updating it)
- Owner-gated: no

The module doc enumerates `capacity`, `faults`, and `fixtures` and omits `announced`, `local_eq`, `skeleton`, `stats`, and `wedge`. A hand-maintained map the code can change without touching is the enumeration Principle 5 forbids; here it is already stale, and the five omitted modules are the ones a newcomer most needs orientation for.

Evidence:

         3	//! Capacity/scheduling stress lives in [`capacity`], connected abort and
         4	//! lifecycle checks in [`faults`], and deterministic tree builders in
         5	//! [`fixtures`].
        27	mod announced;
        28	mod capacity;
        29	mod faults;
        30	mod fixtures;
        31	mod local_eq;
        32	mod skeleton;
        33	mod stats;
        34	mod wedge;

Resolution: Either complete the map with one clause per module (`stats` pins the counters against a dispute oracle; `skeleton`, `wedge`, `local_eq`, and `announced` bridge sessions to the Lean model) or drop the enumeration and let each submodule's own first sentence serve, since the module listing already shows them. Acceptance: every `mod` at tests.rs:27-34 is named in the doc, or the doc enumerates none.

### streaming-tests-2: A driver unit test sits in the streaming tests root while driver.rs keeps an inline test module
- Where: src/tree/mirror/streaming/tests.rs:36-60 (related: src/tree/mirror/streaming/driver.rs:203-238 (outside the partition); src/tree/mirror/streaming.rs:197-204)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read tests.rs:36-60, driver.rs:85-101 and 203-238, streaming.rs:197-204)
- Seen by: structure-prose, api-economics; refutation: confirmed the placement and reframed the api-economics "dissolve it" option (the test pins crate-owned `Client`/`Server` routing, not only `futures`' fail-fast); history: no rationale found (the test and driver.rs's inline module were created together in cbfe1aff)
- Owner-gated: no

`terminal_errors_preempt_parked_peers` exercises `driver::try_join_mapped` alone, with no tree or session; the driver's other two tests live in an inline `#[cfg(test)] mod tests` at driver.rs:203. The three belong together in a `driver/tests.rs` sibling (AGENTS.md's convention); today a reader of driver.rs's tests does not see this one, and the streaming suite opens with a non-session test. The crate-owned claim is that a left error surfaces as `Client` and a right error as `Server`, the routing `descend`'s equal-version short circuit relies on (streaming.rs:198-203); the testdoc names only the fail-fast half, which is `futures::future::try_join`'s contract.

Evidence:

        36	/// Either terminal error preempts a peer which can no longer make progress.
        37	#[test]
        38	fn terminal_errors_preempt_parked_peers() {
        39	    let left = try_join_mapped(
        40	        future::ready(Err::<(), _>("left")),
        41	        MirrorError::<&str, Infallible>::Client,

    driver.rs:
       203	#[cfg(test)]
       204	mod tests {

Resolution: Create `src/tree/mirror/streaming/driver/tests.rs` (`mod tests;` in driver.rs), move this test and the two inline ones there, drop the `try_join_mapped` import from tests.rs, and restate the doc as "A left-side error surfaces as `Client` and a right-side error as `Server`, preempting a parked counterparty." Acceptance: driver.rs has `mod tests;` and no inline module; tests.rs has no `try_join_mapped` reference; the testdoc names the routing.

### streaming-tests-3: The two-Local-endpoints session is constructed at nine sites behind a six-function ladder
- Where: src/tree/mirror/streaming/tests.rs:62-150 (related: tests.rs:105-116, 234-240; capacity.rs:26-36, 198-218; stats.rs:42-61; faults.rs:72-75, 87-90, 135-145, 211-215)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (grep: `Handshaking::start(Local` on 18 lines, nine client/server pairs, across tests.rs (6), faults.rs (6), capacity.rs (4), stats.rs (2); ladder call sites: `streaming_mirror_sides_with_schedule` is called only at tests.rs:65 and :128, both wrappers; `streaming_mirror` only at tests.rs:185; `awk 'length > 100'` over the partition prints exactly faults.rs:72, 73, 74, 87, 88, 89, 97, 211, 212)
- Seen by: structure-prose ([6], [7], [18]), api-economics ([40]); refutation: confirmed; history: no rationale found (seven creating commits between a8b130e6 and 7626543e, each adding a copy for a new instrument)
- Owner-gated: no

The sequence "convert to `StreamingRoot<Local>`, `Handshaking::start(Local, ..).window(WindowConfig::FLOOR)` twice, `run_to_quiescence(drive_streaming(client, server))`, unwrap quiescence and violations" is spelled out at nine sites with different instruments attached, and tests.rs stacks `streaming_mirror_sides` -> `_with_schedule` -> `_with_schedules` plus three convergence-asserting twins that differ only in their message string; two rungs have a single caller. A change to the session's construction touches nine sites, and each suite's distinctive claim is buried under identical setup. The nine lines of 102 to 124 characters in `faults.rs` (unformatted because rustfmt leaves `proptest!` bodies alone) are the same construction spelled inline.

Evidence:

        64	fn streaming_mirror_sides(a: Root, b: Root) -> (Root, Root) {
        65	    streaming_mirror_sides_with_schedule(a, b, Vec::new())
        66	}
        67	
        68	/// Reconcile under an explicit, shrinkable channel-poll schedule.
        69	fn streaming_mirror_sides_with_schedule(a: Root, b: Root, schedule: Vec<u8>) -> (Root, Root) {
        70	    streaming_mirror_sides_with_schedules(a, b, schedule, Vec::new())
        71	}

       106	    let (a, b): (StreamingRoot<Local>, StreamingRoot<Local>) = (a.into(), b.into());
       107	    let client = Handshaking::start(Local, a).window(WindowConfig::FLOOR);
       108	    let server = Handshaking::start(Local, b).window(WindowConfig::FLOOR);
       109	    let ((result, trace), transcript) =
       110	        with_transcript(|| with_trace(|| run_to_quiescence(drive_streaming(client, server))));

    stats.rs:
        43	    let (a, b): (StreamingRoot<Local>, StreamingRoot<Local>) = (a.into(), b.into());
        44	    let a_recorder = Recorder::default();
        45	    let b_recorder = Recorder::default();
        46	    let client = Handshaking::start(Local, a)
        47	        .window(WindowConfig::FLOOR)
        48	        .stats(a_recorder.clone());

Resolution: Give tests.rs one `LocalSession` builder: `LocalSession::new(a, b)` with `.channel_schedule(..)`, `.backend_schedule(..)`, `.kind_capacity(kind, n)`, `.stats()`, `.trace()`, `.transcript()`, and `run(self) -> Outcome`, where `Outcome` carries `Result<Result<(Root, Root), MirrorError<..>>, Quiescence>` plus the requested instruments, with `converged(self) -> Root` and `sides(self) -> (Root, Root)` owning the two `expect`s. Express `transcribed_mirror_sides`, `mirror_with_stats`, `shape_stalls`, `underbuffered_mirror_stalls`, and the inline body of `uncontained_supply_is_rejected_by_streaming` through it; keep `faults.rs`'s own construction only where a `Faulting` or `Failing` wrapper replaces `Local`, via a `floor_start(root)` helper that also wraps the long lines. The same `Outcome` type resolves streaming-tests-11. Acceptance: `grep -c 'Handshaking::start(Local'` over the partition drops to the builder plus the fault-wrapper sites; `streaming_mirror_sides_with_schedule` and `scheduled_streaming_mirror` are gone; `awk 'length > 100' faults.rs` prints nothing.

### streaming-tests-4: Em-dashes in `//` code comments
- Where: src/tree/mirror/streaming/tests.rs:92-94 (related: capacity.rs:301; faults.rs:126; skeleton.rs:533; local_eq.rs:206-207, 334, 364)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for the em-dash character over the nine files, excluding `///` and `//!` lines: exactly these eight lines)
- Seen by: structure-prose; refutation: confirmed; history: no rationale found; the pattern is codebase-wide (the history pass counts roughly 116 such `//` lines under src/), and its source is the owner's global doctrine, not AGENTS.md
- Owner-gated: no

Eight `//` comment lines in the partition use a true em-dash; the doctrine reserves em-dashes for rendered prose and asks for colons, semicolons, or spaced double-hyphens in code comments. Fixing the eight leaves the convention unenforced elsewhere; the useful disposition is a small lint or a codebase-wide sweep, not a partition edit.

Evidence:

        92	        // `Local` is infallible, so the session's only inhabited errors are
        93	        // violations — which two honest local endpoints must never speak.

Resolution: Replace each with a colon, semicolon, or ` -- `, and consider a `tools/` check (or an addition to the existing prose linters) that rejects the character on `//` lines that are not `///` or `//!`. Acceptance: the grep is empty over the partition, or the check exists and passes.

### streaming-tests-5: Two doc comments in tests.rs describe code that no longer exists
- Where: src/tree/mirror/streaming/tests.rs:99-105 (related: src/tree/mirror/streaming/tests.rs:176-180)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git log -L99,105` lists 97dfbcdf (doc and `<T: Send + Sync + 'static>` added together), b524e406, 48bc31df (signature erased, doc unchanged); `git log -L176,180` lists only a8b130e6; `git grep -w 'Uncertain\|Closing' a8b130e6 -- src/tree/mirror/streaming` hits `convert.rs:18,230,235,241`; a word-boundary grep for `uncertain\|Closing` over streaming production code at HEAD returns nothing)
- Seen by: structure-prose ([0], [1]), api-economics ([39]); refutation: confirmed both; history: deliberate-but-expired for both. Attribution correction from the history pass: the `uncertain`/`Closing`/`Complete` vocabulary at 179-180 was streaming's own message vocabulary when written (a8b130e6, 2026-07-09) and was collapsed to `Reply` by 7126e489 (2026-07-14), seven weeks before the alternating protocol retired; it is not V1 leakage
- Owner-gated: no

Two paragraphs describe mechanisms the code no longer has. The doc on `transcribed_mirror_sides` (99-105) justifies a type parameter the function lost in the payload-erasure commit, and misleads about why payload twins work (they work because `Root` carries erased payloads, not because of generics). The doc on `converges_on_leaf_parent_dispute` (176-180) explains the leaf-parent merge in terms of `uncertain`, `Closing`, and `Complete`, none of which streaming code speaks today. AGENTS.md's hard rule: nothing refers to code that no longer exists; a testdoc held to accuracy is a bug when inaccurate. Two prose sweeps whose stated goal covered the second site (44724ad0 and the V1-retirement re-denomination) missed it.

Evidence:

       102	/// The skeleton-bridge harness ([`skeleton`]): generic over the payload type
       103	/// so payload-perturbation twins (same paths, different contents) can run
       104	/// through the identical machinery.
       105	fn transcribed_mirror_sides(a: Root, b: Root) -> (Root, Root, Trace, Transcript) {

       179	/// The responder's closing `uncertain` lists its leaves, and the leaf-height
       180	/// `Closing`/`Complete` words carry the difference in both directions.

Resolution: 102-104: "The skeleton-bridge harness ([`skeleton`]). Payload twins ([`announced`]) differ only in leaf contents, which the erased `Root` carries opaquely, so both run through identical machinery." 179-180: delete (the invariant sentence at 176-177 stands alone) or restate against today's code: the leaf-parent answerer merge-joins both leaf listings, supplying its exclusive leaves and issuing a leaf request for each it lacks. Acceptance: `grep -n 'generic over the payload\|uncertain\|Closing' src/tree/mirror/streaming/tests.rs` is empty.

### streaming-tests-6: `arb_oracle_pair` omits the deep-spine generator built to reach divergence below the root
- Where: src/tree/mirror/streaming/tests.rs:168-174 (related: src/tree/arb.rs:232-291; src/tree/tests.rs:1201; capacity.rs:356-385)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep: `arb_deep_divergent_pair` is used only at its definition, arb.rs:244, and at src/tree/tests.rs:1201)
- Seen by: blind-spots ([24]); refutation: reframed (the original claim that depth reaches the streaming walk only through hand fixtures is false: `scheduled_structured_disputes_match_oracle` draws pyramids to depth 6 and boundary fans to depth 31 against `join_oracle`; what the deep-spine family adds is its zero-width, subset, identical, and ceiling-only merges at drawn depth); history: no rationale found (the strategy dates from 83edcd94; the deep generator arrived later in 48d5253d for the join suite; 69c7ac80 retargeted the differential to `Tree::join` without touching the strategy)
- Owner-gated: no

The central differential samples content-addressed pairs whose keys scatter at the root fan (arb.rs:235-237: "Content-addressed generators cannot produce this shape"). `arb_deep_divergent_pair` draws a shared spine of depth 0..32 with novelty widths that include zero, so subset, identical, and ceiling-only merges at depth are sampled; the join suite uses it and the streaming differential does not. Adding the arm costs nothing and widens the walk's oracle coverage to the family the generator exists for.

Evidence:

       168	fn arb_oracle_pair() -> impl Strategy<Value = (Root, Root)> {
       169	    prop_oneof![
       170	        4 => arb_divergent_pair(),
       171	        2 => (arb_tree_root(0, 0..=8), arb_tree_root(1, 0..=8)),
       172	        1 => arb_tree_root(0, 0..=8).prop_map(|root| (root.clone(), root)),
       173	    ]
       174	}

Resolution: Add `2 => arb_deep_divergent_pair()` (and optionally `arb_wide_divergent_pair()`) as arms and name the deep-spine family in the doc at 163-167; commit any seed that appears. Acceptance: the arm is present and `streaming_matches_join_oracle` is green.

Construction: none needed beyond the strategy edit; a failure, if any, writes to `proptest-regressions/tree/mirror/streaming/tests.txt`.

### streaming-tests-7: Redaction at depth is pinned by one two-leaf fixture; no generated family combines depth with redaction
- Where: src/tree/mirror/streaming/tests.rs:194-196 (related: src/tree/arb.rs:132-172, 244-291; stats.rs:198-218; src/tree/mirror/streaming/backend.rs:127-129)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read `arb_divergent_pair` at arb.rs:132-172: forgets are drawn over content-addressed shared keys; `arb_deep_divergent_pair` at 244-291 draws no forgets)
- Seen by: blind-spots ([23]); refutation: reframed (redaction under a depth-1 shared prefix is reached by hash collision, roughly 1/256 per shared pair, so "never" is overstated; the pruned-to-nothing reply the backend doc names is exercised at the root fan; the deletion filter has its own differential in `unknown/tests.rs`); history: no rationale found
- Owner-gated: no

The only deterministic redaction below the root in the streaming suite is `leaf_parent_redaction_pair` (one leaf forgotten, one concurrent sibling), and the only generated redaction is `arb_divergent_pair`'s, whose forgets land at content-addressed keys. No strategy draws a shared spine of chosen depth together with a subset of forgets, so a redaction that empties a whole disputed child below the root, or two sides redacting different siblings under one leaf parent, is unsampled. The join oracle is available and cheap, so the family is stateable as a property.

Evidence:

       194	#[test]
       195	fn honors_redaction_under_leaf_parent_dispute() {
       196	    let (a, b, expected) = leaf_parent_redaction_pair();

Resolution: Add `arb_deep_redaction_pair` beside `arb_deep_divergent_pair` in `arb.rs` (a shared spine of drawn depth carrying k drawn shared leaves; each side forgets a drawn subset on its own party and adds drawn concurrent extras), include it as an arm of `arb_oracle_pair`, and add two fixtures: (a) both sides hold prefix P, side a holds child Q under P with k >= 2 leaves that side b forgot (b's request for Q prunes to nothing on a); (b) each side forgets the other's sibling under one 31-byte prefix. Acceptance: `arb_oracle_pair` draws forgets under a shared prefix of depth >= 1; the two fixtures are committed with `join_oracle` as the expectation in both orientations.

Construction: For (a): `grown(None, 0, 1, &(), &[spine, q1, q2])` with `spine = path_at(&[0; 32])`, `q1 = path_at(&[0, 0, 1, 0x10])`, `q2 = path_at(&[0, 0, 1, 0x11])`; side a is that node; side b is the same node with `q1` and `q2` removed via `act(node, [(q1, v1, Action::Forget), (q2, v2, Action::Forget)], ..)` ticked on party 1, plus one concurrent extra under `[0, 0, 2]` on party 1. Expected is `join_oracle(a, b)` in both orientations.

### streaming-tests-8: Campaign roster IDs and default-dialect vocabulary in the bridge and probe docs
- Where: src/tree/mirror/streaming/tests/announced.rs:1-2 (related: announced.rs:41; skeleton.rs:18-19, 21, 506; wedge.rs:1, 4, 9, 118; local_eq.rs:1, 12-13, 142, 259; capacity.rs:244, 280; fixtures.rs:31; faults.rs:61; outside the partition: materialized/transcript.rs:10-11, materialized/progress.rs:82, 93, 114, 208, materialized/progress/tests.rs:62, 80, 102, 121)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep enumerates every site; `grep -rnE '^(theorem|def|lemma|abbrev)\s+(B5|T3|F4)\b' formal/lean` returns nothing, so none is a Lean declaration name; `finding #7` resolves to formal/MODEL.md:36, 41, 507, 636 and formal/PROGRESS.md:981; `F4` resolves only to `.agent-notes/`; `B5` is defined in formal/doc/exposition.typ:1132 ("the fifth of the campaign's numbered Rust bridge tests") and named in Lean docstrings)
- Seen by: structure-prose ([3]), blind-spots ([31]), api-economics ([37], [51]); refutation: confirmed; history: no rationale found (ad89a63e removed the document pointers and kept the tags, introducing "charter" and "adjudicated" as the restatement; b204fe56 and ccdd4c5d normalized the Lean citations in local_eq.rs and left the tags)
- Owner-gated: no

The bridge suites identify what they test by tags from the mux campaign roster: `B5`, `T3`, `F4`, `finding #7`, the ordinals `Bridge 1/2/3`, and the process words `adjudicated`/`adjudication` and `charter`. None is a Lean theorem or definition name (the sanctioned citation form), `finding #7` resolves only in formal/MODEL.md (which AGENTS.md forbids citing from code) and an agent note, and `F4` resolves nowhere in the tree. The Lean names beside them (`wc_impossibility`, `Mux.wedge`, `viewEnc`, `LocalEq`) already carry the meaning. The same files carry the dialect tells the owner names: `knobs` (fixtures.rs:31), `genuine`/`genuinely` (faults.rs:61, local_eq.rs:12-13), and the colon-fronted fragment "Deviations from the Lean, recorded:" (skeleton.rs:21).

Evidence:

    announced.rs:
         1	//! Bridge 3: announced-skeleton reconstruction — the payload-independence
         2	//! bridge B5.

    wedge.rs:
         4	//! The mux impossibility theorem T3 (`wc_impossibility`) quantifies over one
         9	//! trees whose dispute skeleton IS the wedge (adjudication repair F4: for
        10	//! impossibilities, realizability flows from Rust to the model).

    capacity.rs:
       244	/// Probe (model finding #7): a lone parent scope stalls the real encoder

    skeleton.rs:
        18	//!   reconstruction bridge B5 asks for: payload-erased frame contents
        19	//!   determine the announced skeleton, the fact charter locality rests on.

Resolution: Name the thing at each site and keep the Lean names: "the payload-independence bridge (announced-skeleton reconstruction)" for B5; "the mux impossibility theorem `wc_impossibility`" for T3; drop "adjudication repair F4" and say "for an impossibility, realizability flows from Rust to the model"; "Parent-placement probe" for "finding #7"; drop the `Bridge N` ordinals (each file's first sentence already names its bridge); "the fact the locality theorems rest on" for "charter locality"; "the model's" for "adjudicated"; "parameters" for "knobs"; "A malformed reply" for "A genuine malformed reply"; "held"/"absent" for "genuinely held"/"genuinely absent". Sweep the out-of-partition sites in the same pass so production and tests keep one vocabulary. Acceptance: `grep -rnE '\b(B5|T3|F4)\b|Bridge [0-9]|finding #[0-9]|adjudicat|charter|knob|genuine' src/tree/mirror/streaming` is empty; every formal citation is a Lean theorem or definition name.

### streaming-tests-9: The announced.rs module doc records a build-time observation as narrative
- Where: src/tree/mirror/streaming/tests/announced.rs:26-32 (related: skeleton.rs:641-648; src/tree/mirror/streaming/materialized.rs:841-850)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (cat of `proptest-regressions/tree/mirror/streaming/tests/announced.txt` shows two `cc` lines; read materialized.rs:841-850: `tokio::select!` with no `biased;`)
- Seen by: structure-prose ([15]), api-economics ([49]); refutation: confirmed; history: deliberate-and-holds for the mechanism (97dfbcdf recorded the finding here on purpose, and the unbiased select is still true); the parenthetical is a trim compatible with that decision
- Owner-gated: no

The scoping rationale ends with a diary entry ("observed while building this bridge: the committed regression seed reorders..."). The durable fact is one sentence: the claim is per channel because `complete_initiator`'s terminal `tokio::select!` is unbiased, so the global order varies run to run. The history of how that was found belongs in git (where 97dfbcdf's message already records it), "the committed regression seed" (singular) disagrees with the two seeds in announced.txt, and skeleton.rs:641-648 explains the same mechanism a second time, so the two copies can drift.

Evidence:

        26	//! adversarially, and the real global publication order is not even a
        27	//! function of the trees — `complete_initiator`'s terminal `tokio::select!`
        28	//! is unbiased, so its branch order draws tokio's thread-local RNG and
        29	//! reorders the tail of otherwise identical back-to-back runs (observed
        30	//! while building this bridge: the committed regression seed reorders the
        31	//! final absorb-side events between two runs of the SAME trees; the
        32	//! per-channel projections are unaffected).

Resolution: Cut from "(observed" through "unaffected)". Keep the mechanism once, at `trace_channels` (skeleton.rs:641-648), and have announced.rs say "the claim is per channel, not the global interleaving ([`trace_channels`] explains why)". Acceptance: no "observed while" phrasing; one explanation of the unbiased `select!` in the partition.

### streaming-tests-10: The reply capture is called a 'wire transcript'
- Where: src/tree/mirror/streaming/tests/announced.rs:81-85 (related: tests.rs:99-100; announced.rs:10-11, 41, 50; skeleton.rs:15, 483, 503; src/tree/mirror/streaming/materialized/transcript.rs:1-20)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read transcript.rs:1-20: the capture point is `Work::respond`, recording each `Reply` reduced to labels, with no link involved)
- Seen by: blind-spots ([32]); refutation: confirmed and lowered to nit (the transcript module doc already names its capture point; only the assertion message and a few doc phrases mislead); history: deliberate-and-holds (97dfbcdf chose reply granularity with byte-budget frame splitting already present; the model's premise is stated over channel operations)
- Owner-gated: no

`transcribed_mirror_sides` runs two in-process walks with no link; the transcript captures `Reply` values at `Work::respond`. Calling that "the payload-erased per-stream wire transcript" and asserting it payload-independent is true at reply granularity; on a link the proxy splits supply runs by byte budget, so frame count depends on payload size, and a future reader could take the message as evidence about frame-level behavior. The transcript module doc itself says "what actually crosses the wire", so the imprecision starts outside this partition.

Evidence:

        81	        prop_assert_eq!(
        82	            transcript_streams(&transcript),
        83	            transcript_streams(&twin_transcript),
        84	            "the payload-erased per-stream wire transcript is payload-independent"
        85	        );

Resolution: Say "reply transcript" (or "the walk's outgoing replies, payload-erased") in the assertion message and the listed doc phrases, and add one clause where the bridge is introduced noting that link-level framing of supplies is byte-budgeted and out of this bridge's scope. Acceptance: no assertion message or doc in the partition calls the reply capture a wire transcript.

### streaming-tests-11: Stall probes report violations and livelocks as completion
- Where: src/tree/mirror/streaming/tests/capacity.rs:26-36 (related: capacity.rs:191-194, 198-218, 238-242, 265-277, 303-329; src/testing.rs:345-394)
- Class / severity / confidence: correctness / high / high
- Provenance: assessed (read `run_to_quiescence`'s signature `Result<F::Output, Quiescence>` with `Quiescence::{Stalled, PollBudget}` at testing.rs:345-394; `F::Output` here is the session's `Result<(Root, Root), MirrorError<..>>`; the `matches!` arms admit only `Err(Quiescence::Stalled)`)
- Seen by: api-economics ([35]); refutation: confirmed and proposed lowering to medium (a test-probe weakness rather than a production defect, and the probed shapes are compared to the oracle at default capacity by other tests); history: no rationale found (the probes are load-bearing "[checked]" evidence in the parent-placement note; the positive `stalls` assertions are unaffected and the negative ones are the gap, so a three-way probe strengthens the recorded argument)
- Owner-gated: no

The severity stays at high because the review's rubric names "a harness bug that masks failures" as high, and this is one. `underbuffered_mirror_stalls` and `shape_stalls` collapse three outcomes (`Ok(Ok(_))` completion, `Ok(Err(violation))`, `Err(Quiescence::PollBudget)`) into `false`, so the five assertions documented as "must complete" (`!stalls_under_any_schedule(..)` at 265-269, 274-277, 308-312, 323-329 and `!underbuffered_mirror_stalls(a, b, 254)` at 191-194) pass if the session dies with a protocol violation or exhausts the million-poll budget. The completion arm also never compares the reconciled roots to `join_oracle`, and the other oracle comparisons run at the default capacity, not at capacity 1 or 254, so a regression specific to the constrained capacity is invisible. The refutation's point that no production defect is known is true and does not change the classification: the cheapest passing artifact is not the intended one.

Evidence:

        30	    with_kind_capacity(QueueKind::AssemblyLevelReturns, capacity, || {
        31	        matches!(
        32	            run_to_quiescence(drive_streaming(client, server)),
        33	            Err(Quiescence::Stalled)
        34	        )
        35	    })

       265	    assert!(
       266	        !stalls_under_any_schedule(&internal_fan(3), 1),
       267	        "fan = cap + 2 must complete: the model's tighter pdelay boundary \
       268	         is not realizable in the sequential encoder"
       269	    );

Resolution: Replace the boolean probes with a three-way outcome: `fn outcome(pair, capacity, schedules) -> Result<Result<(Root, Root), MirrorError<..>>, Quiescence>` (or the `Outcome` of the `LocalSession` builder in streaming-tests-3), with `stalls(o) = matches!(o, Err(Quiescence::Stalled))` and `completes(o)` returning the roots so the caller can compare them to `join_oracle`. Rewrite `stalls_under_any_schedule` as `any(stalls)` and add `completes_under_every_schedule` as `all(completes)` for the negative sites; treat `Ok(Err(_))` and `Err(Quiescence::PollBudget)` as failures with their own messages. Acceptance: inject `Fault::Reply(Violation::UnexpectedQuery)` into the `internal_fan(3)` session (or temporarily make the completion path return `Ok(Err(..))`) and confirm `parent_delay_single_parent_boundary` goes red; restore and confirm green; every `!stalls` site is a `completes` site whose roots are compared to `join_oracle`.

Construction: In `parent_delay_single_parent_boundary`, wrap the server in `Faulting::new(server, 0, Some(Fault::Reply(Violation::UnexpectedQuery)))` inside `shape_stalls`; today the test stays green because `Ok(Err(..))` reads as "did not stall".

### streaming-tests-12: Poll-schedule literals are copied across files with their clamp and length unexplained
- Where: src/tree/mirror/streaming/tests/capacity.rs:42-52 (related: capacity.rs:118, 120, 177, 221-235; tests.rs:198-203; src/tree/mirror/streaming/channel/instrumented.rs:308-318; src/tree/mirror/streaming/backend/local/adversarial.rs:115-125)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: `16_384` on 13 lines of capacity.rs and `2_048` on two lines of tests.rs; read both consumers: `delay.min(2)` at instrumented.rs:316 and adversarial.rs:123, and `.get(current.step).copied().unwrap_or(0)` at instrumented.rs:314 and adversarial.rs:121)
- Seen by: structure-prose ([10]), blind-spots ([34]), api-economics ([45]); refutation: confirmed (corrected the count from eleven to 13 lines); history: no rationale found (the `% 5` row was written in 0ad9077a against a clamp that already existed)
- Owner-gated: no

`assert_capacity_case` inlines the first three rows of `probe_schedules()`; tests.rs carries a fourth copy at length 2_048; `vec![2; 16_384]` recurs at 118, 120, and 177. Both consumers clamp every delay to 2, so the `% 5` row at 232 runs as 0,1,2,2,2 (a reader infers five delays and gets three), and an exhausted schedule runs the rest of the session undelayed with no signal, so whether 16_384 covers the largest fixtures is unmeasured. Named constants over magic numbers; one provider over four copies.

Evidence:

        42	    let schedules = [
        43	        (Vec::new(), Vec::new()),
        44	        (
        45	            vec![2; 16_384],
        46	            (0..16_384).map(|step| (step % 3) as u8).collect(),
        47	        ),
        48	        (
        49	            (0..16_384).map(|step| (step % 3) as u8).collect(),
        50	            vec![2; 16_384],
        51	        ),
        52	    ];

       232	        ((0..16_384).map(|s| (s % 5) as u8).collect(), Vec::new()),

Resolution: In tests.rs define `const MAX_DELAY: u8 = 2` (asserted equal to the consumers' clamp, or exported from them), `const SCHEDULE_LEN: usize = 16_384` with a one-line derivation, a `fn round_robin(modulus: u8) -> Vec<u8>`, and one `fn standard_schedules()` used by `assert_capacity_case`, `probe_schedules`, and `honors_redaction_under_leaf_parent_dispute`; rewrite or delete the `% 5` row. Optionally have `with_schedule` return the steps consumed so a test can assert `consumed <= SCHEDULE_LEN`. Acceptance: `grep -rn '16_384\|2_048' src/tree/mirror/streaming/tests*` hits only the constant definitions; the `% 5` row is gone or stated in terms of `MAX_DELAY`.

### streaming-tests-13: Two testdocs understate or mislabel what their bodies check
- Where: src/tree/mirror/streaming/tests/capacity.rs:113-115 (related: capacity.rs:154-168, 363-366, 375-383; src/tree/mirror/streaming/channel.rs:31-33)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read the two bodies; `QueueKind::ALL` is documented "Every materialized semantic edge, for its coverage assertions." at channel.rs:31 and excludes the proxy kinds)
- Seen by: structure-prose ([21]); refutation: confirmed; history: no rationale found (both docs are verbatim from ddc9a2d8; the oracle comparison at 383 existed from the start)
- Owner-gated: no

`capacity_stress_covers_every_queue_role` says "Every named, height-carrying queue" while the body iterates `QueueKind::ALL` (every materialized semantic edge, proxy kinds excluded) and additionally asserts that typed heights survive observation (154-162) and that some sender saw backpressure (163-168), neither mentioned. `scheduled_structured_disputes_match_oracle`'s doc says the disputes "terminate" while the body asserts equality with `join_oracle` (the stronger claim its name states). A doc narrower than its body hides coverage a future edit may drop; AGENTS.md holds the testdoc to the behavior and invariant the test protects.

Evidence:

       113	/// Every named, height-carrying queue is exercised at its documented capacity.

       363	    /// Structured disputes terminate under independently shrinkable channel
       364	    /// and Local-backend poll schedules.
       383	        prop_assert_eq!(actual, expected);

Resolution: 113: "Every materialized queue role is constructed, carries traffic, and runs at its documented capacity; recursive roles keep their typed heights, and the scheduled run applies backpressure." 363-364: "Structured disputes converge to the join oracle under independently shrinkable channel and Local-backend poll schedules." Acceptance: each sentence in the two docs corresponds to an assertion in the body and vice versa.

### streaming-tests-14: Statements that carry nothing: a root returned to be dropped, a bound implied by the line above it, a conditional implied by reflexivity
- Where: src/tree/mirror/streaming/tests/capacity.rs:116-123 (related: capacity.rs:144-151; local_eq.rs:120-123; src/tree/mirror/streaming/channel/instrumented.rs:77-86, 185-193, 270)
- Class / severity / confidence: vestigial / nit / high
- Provenance: assessed (read `with_observation<R>` at instrumented.rs:270; `sent()`/`received()` at 77-86 and the `mpsc::channel(effective_capacity)` construction at 193; `view_enc` is a pure function of its arguments)
- Seen by: structure-prose ([13]), blind-spots ([27] items 2 and 3), api-economics ([46]); refutation: confirmed; history: no rationale found
- Owner-gated: no

(1) `capacity_stress_covers_every_queue_role` threads the first session's root out of the `with_observation` closure only to `drop(pair)`; the closure can return `()`. (2) `high_water <= expected` at 148-151 follows from `effective_capacity == expected` at 144 and tokio's bounded buffer (occupancy is sends minus receives, and a send completes only with room), so it cannot fail independently. (3) local_eq.rs:120 asserts `local_eq(p, s0, s0)`, and 121-123 asserts `local_eq(p, s0, s1)` only when `s0 == s1`, which is the same evaluation. Each statement costs a reader a "why is this here" without protecting anything (Principle 3).

Evidence:

       116	    let (pair, report) = with_observation(|| {
       117	        let (a, b) = pyramid_pair(&[4, 4, 2], 2, LeafOrder::Interleaved);
       118	        let pair = scheduled_streaming_mirror(a, b, vec![2; 16_384]);
       119	        let (a, b, _) = leaf_parent_dispute_pair();
       120	        scheduled_streaming_mirror(a, b, vec![2; 16_384]);
       121	        pair
       122	    });
       123	    drop(pair);

       148	        assert!(
       149	            stats.high_water <= expected,
       150	            "queue role {kind:?} exceeded its effective capacity: {stats:?}"
       151	        );

    local_eq.rs:
       120	        prop_assert!(local_eq(p, &decoded_0.skel, &decoded_0.skel));
       121	        if decoded_0.skel == decoded_1.skel {
       122	            prop_assert!(local_eq(p, &decoded_0.skel, &decoded_1.skel));
       123	        }

Resolution: Return `()` from the closure and delete `drop(pair)`; delete 148-151; delete local_eq.rs:121-123, and if line 120 goes too, drop "and `local_eq` is reflexive on decoded skeletons" from the doc at 99-100. Acceptance: none of the three statements is present.

### streaming-tests-15: Magic numbers where a named constant exists or is owed
- Where: src/tree/mirror/streaming/tests/capacity.rs:136-143 (related: capacity.rs:183, 188, 192; fixtures.rs:396; announced.rs:75; skeleton.rs:21-23, 79-80; src/tree/mirror/streaming/window.rs:132)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -n 'const FAN' window.rs` shows `pub(crate) const FAN: usize = 256;` at 132; queues.rs:33 imports `window::FAN`)
- Seen by: structure-prose ([12]); refutation: confirmed (adds the actual slot bounds: `Outside`/`Reversed` need shared <= 254, `Interleaved` needs 2*shared <= 255) and raised skeleton.rs:21-23 as a new site; history: no rationale found (FAN was already `pub(crate)` when the 256 literal was written in 2fe56257)
- Owner-gated: no

The expected capacity of the two constant-fan queues is written `256` although `window::FAN` is `pub(crate)` and sizes those very queues; 254 and 253 at 183-192 derive from the same constant; skeleton.rs:23 and :80 state "`FAN = 256`" in prose, a hand-maintained number that rots if the constant moves. `LeafOrder::slots` asserts `(1..=100).contains(&shared)` with no statement of where 100 comes from, and announced.rs:75 perturbs payloads with the bare `0x0abe_1e57ed_u64`.

Evidence:

       141	            QueueKind::AssemblyLevelReturns | QueueKind::TerminalLeafResolutions => 256,

    skeleton.rs:
        21	//! Deviations from the Lean, recorded: the mirror carries `scopes` and
        22	//! `rootH` only. `Skel.fan` and `Skel.capLevel` are model configuration with
        23	//! a single Rust value each (`FAN = 256` and the margin-0 discipline), so

    fixtures.rs:
       396	        assert!((1..=100).contains(&shared));

Resolution: Import `window::FAN` and write `FAN`, `FAN - 2`, `FAN - 3` with the derivation stated once at 183-192; cite `FAN` by name without its value in skeleton.rs; state the slot bound as a named constant with its reason; name the twin payload (`const TWIN_PAYLOAD: u64 = ..;` "any value other than 0"). Acceptance: `grep -n '\b256\b' capacity.rs skeleton.rs` matches nothing that states the value; the three constants have names.

### streaming-tests-16: A measured stall-boundary table lives in a comment while the test pins one cell
- Where: src/tree/mirror/streaming/tests/capacity.rs:299-312 (related: capacity.rs:244-278)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read the comment and the two assertions below it)
- Seen by: structure-prose ([11]), api-economics ([42]); refutation: confirmed; history: no rationale found (0ad9077a added the tally and the two P=4 assertions together; the parent-placement note cites the 253/254 and 9/27/81 pins but never the P=6/8/12 cells, so those numbers exist only in this comment)
- Owner-gated: no

The control comment reports four measured boundaries (P=4 at C=1, P=6 at C<=3, P=8 at C<=4, P=12 at C<=6) as confirmation of the per-scope law, but the assertions pin only P=4 at C=1 and C=2; `parent_delay_single_parent_boundary` likewise states "fan <= cap + 2" and pins cap 1 and one point at cap 2. A number in prose is a hypothesis until a committed check moves on it (Principle 8), and a hand-maintained tally rots silently (Principle 5). Each cell costs at most five sessions of a tree under 40 cells.

Evidence:

       299	    // Control: growing the count of fan-3 parents under ONE root scope grows
       300	    // the ROOT's own fan, so the per-scope law (fan ≤ cap + 2) predicts the
       301	    // stall boundary — confirmed empirically (P=4 stalls at C=1, P=6 at C≤3,
       302	    // P=8 at C≤4, P=12 at C≤6: exactly P > C + 2 throughout).
       303	    assert!(
       304	        stalls_under_any_schedule(&parents_of_three(4), 1),

Resolution: Replace the tally with a loop over `[(4, 1), (6, 3), (8, 4), (12, 6)]` asserting `stalls(parents_of_three(p), c)` and `completes(parents_of_three(p), c + 1)` (using the three-way probe from streaming-tests-11), and likewise `for cap in 1..=6` in `parent_delay_single_parent_boundary`; reduce the comment to the law. Acceptance: every (P, C) pair named in the two tests' prose is asserted by the body, or absent.

### streaming-tests-17: Two finite fault rosters are sampled 256 times instead of enumerated, around a bare phase-count literal
- Where: src/tree/mirror/streaming/tests/faults.rs:41-58 (related: faults.rs:63-68, 115-119; src/tree/mirror/streaming/driver.rs:161-171)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read the strategies and the driver schedule; the refutation pass verified that no `PROPTEST_CASES` override exists in `.config/nextest.toml` or the justfile)
- Seen by: blind-spots ([26]), api-economics ([41]); refutation: reframed (the per-run miss probability under sampling is small, about 2% for one of the 64 (violation, side, step) cells and negligible for the ten lie cells, so the gain is determinism, legibility of the step bound, and session count, not coverage risk); history: no rationale found. Interplay: enumerating orphans seed lines 7, 8, 12, and 15 of faults.txt (see streaming-tests-20)
- Owner-gated: no

`arb_connected_violation` and `arb_greeting_lie` are `prop_oneof![Just(..)]` over 2 and 5 values; `greeting_lies_classify_exactly` draws a 10-point space 256 times (each case one 32-height comb session plus a baseline session on the tolerated branch), and `connected_violation_aborts_without_mutating_root` draws 32 points per direction 256 times, two sessions per case. Nothing is shrinkable or unbounded; exhaustive loops cover every point deterministically in roughly 74 sessions instead of about 1000. The step bound `0usize..=15` is the driver's reply-phase count (driver.rs:164-168) left as a literal, and it undercounts by one (streaming-tests-19).

Evidence:

        41	fn arb_connected_violation() -> impl Strategy<Value = Violation> {
        42	    prop_oneof![
        43	        Just(Violation::UnexpectedQuery),
        44	        Just(Violation::UncontainedSupply),
        45	    ]
        46	}

        66	        server_steps in 0usize..=15,
        67	        client_steps in 0usize..=15,

Resolution: Give `Violation` (or a test-local const) and `GreetingLie` an `ALL` array; rewrite both tests as plain `#[test]`s with nested loops over `ALL x [true, false]` and `ALL x 0..=REPLY_PHASES x side`, where `REPLY_PHASES` is a named constant tied to the driver's schedule; keep `materialized_backend_failures_are_fail_fast` as a proptest. Acceptance: both tests are deterministic enumerations; faults.rs has one `proptest!` block; the orphaned seed lines have an owner ruling.

### streaming-tests-18: The 'input roots remain untouched' assertions compare a value with its own aliased clone
- Where: src/tree/mirror/streaming/tests/faults.rs:61-100 (related: faults.rs:1, 103-122, 188; src/tree.rs:108-135; src/tree/typed/untyped.rs:21-23, 684-695; src/tree/mirror/streaming/backend/local.rs:232-237; src/tests.rs:550)
- Class / severity / confidence: test-quality / medium / high
- Provenance: assessed (read: `tree::Root` has private `ceiling` and `root` fields and `PartialEq` compares them (tree.rs:108-135); `Node` is `Arc<NodeInner>` and `Node::eq` is `ptr_eq || hash` (untyped.rs:21-23, 694); the sessions receive `StreamingRoot::from(client_root.clone())`, and `From<tree::Root> for Root<Local>` moves the clone (backend/local.rs:232-237); nothing between the snapshot and the assertion holds a reference to the originals)
- Seen by: structure-prose ([4]), blind-spots ([27] item 1), api-economics ([36]); refutation: confirmed and raised faults.rs:1 ("lifecycle atomicity") as the same claim at module level; history: no rationale found (tautological from birth in ddc9a2d8: the sessions already received clones)
- Owner-gated: no

Both tests snapshot `before = (client_root.clone(), server_root.clone())`, hand further clones to the sessions, and close with `prop_assert_eq!((client_root, server_root), before)`. `Root` is an immutable `Arc` tree with no `&mut` path from a session to the caller's value, so the two tuples alias the same heap nodes and the assertion is decided by the types. The testdocs ("remain untouched", "with both input roots untouched") and the module doc ("lifecycle atomicity") advertise an atomicity property the tests do not test, and the walk tier cannot test it, because a session's only output is its `Ok` value. The commit-tier atomicity claim is pinned elsewhere (`uncontained_supply_fails_gossip_and_poisons_the_link`, src/tests.rs:550). A guard earns its place by naming a constructible failure it catches (Principle 3); an inaccurate testdoc is a bug in the test.

Evidence:

         1	//! Connected-session abort routing and lifecycle atomicity.

        61	    /// A genuine malformed reply crosses the fully connected driver as its
        62	    /// detected violation while both materialized input roots remain untouched.

        71	        let before = (client_root.clone(), server_root.clone());

       100	        prop_assert_eq!((client_root, server_root), before);

Resolution: Delete the `before` snapshots and the two closing `prop_assert_eq!`s (71, 100, 122, 188); drop "while both materialized input roots remain untouched" (61-62), "with both input roots untouched" (108-109), and "and lifecycle atomicity" (1); rename the first test to `connected_violation_aborts_with_its_typed_error`. If a walk-tier atomicity statement is wanted, its observable is the session's `Output` on the failing path, not the caller's clone. Acceptance: no assertion in faults.rs compares a pre-session clone of an input root to itself; the docs claim only error routing, classification, and unchanged reconciliation under tolerated lies.

### streaming-tests-19: The fault-injection step range stops one phase short of the terminal reply on both sides
- Where: src/tree/mirror/streaming/tests/faults.rs:66-67 (related: src/tree/mirror/streaming/driver.rs:161-171; src/tree/mirror/streaming/testing/faulting.rs:164-168, 248-270, 288-299, 403-436; src/tree/mirror/streaming/protocol.rs:148-151; proptest-regressions/tree/mirror/streaming/tests/faults.txt:8)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: verified (read the driver schedule and every `Faulting` phase impl; `git log -S<hash>` places seed line 8 (`server_steps = 16`) in ddc9a2d8, and `git show ddc9a2d8:...faults.rs` shows `server_steps in 0usize..=15` in the same commit)
- Seen by: blind-spots ([25]); refutation: confirmed; history: no rationale found, and the evidence strengthens the finding: `complete_responder` already injected at that commit, and a seed pinned at exactly the excluded step means proptest shrank toward 16 and could go no lower, so steps <= 15 passed and 16 failed. Inference (the history pass's, labeled as such): the range was narrowed around the failure rather than the failure diagnosed
- Owner-gated: no

The connected schedule gives each side seventeen outgoing phases: the opening (step 0), fifteen loop replies (1..=15), then the initiator's sixteenth reply at height `Z` (protocol.rs:148-151) and the responder's `complete_responder` (16). `fault_phase` decrements `remaining` on every non-injecting phase, `connect` does not, and `complete_responder` injects at `remaining == 0` (faulting.rs:416), so step 16 is exactly the leaf-height terminal phase on either side, where leaf supplies and leaf requests pair. The strategy draws `0usize..=15`, so that phase is never corrupted through the connected driver, and the one committed seed that names step 16 records a failure there.

Evidence:

        66	        server_steps in 0usize..=15,
        67	        client_steps in 0usize..=15,

    faults.txt:
         8	cc f7515199a92a2d04d20b953c417021f4771ca93cc0ec4603cf51d6da120fdb8b # shrinks to server_steps = 16, client_steps = 0

Resolution: Widen both ranges to `0usize..=16` (or enumerate them per streaming-tests-17) and run. If the terminal phase turns out unreachable for the comb fixture, the `Ok(_)` arm already fails the test and that is the finding; if the fault is not surfaced as the expected violation, that is a `Faulting` or driver finding. Either way the disposition is recorded, not the range narrowed. Acceptance: step 16 is drawn or enumerated for both sides and the test is green, or the failure is filed.

Construction: Set `server_steps = 16` and `violation = Violation::UnexpectedQuery` against `full_depth_comb_pair(2, LeafOrder::Interleaved)`; the fault lands in `complete_responder`'s reply stream and must surface as `MirrorError::Client(MaterializedError::Violation(Violation::UnexpectedQuery))`.

### streaming-tests-20: Two committed seed comments name values the current strategies cannot generate
- Where: proptest-regressions/tree/mirror/streaming/tests/faults.txt:7-8 (related: faults.txt:11-15; proptest-regressions/tree/mirror/streaming/tests/stats.txt:7; faults.rs:41-46, 66-67; src/tree/mirror/streaming/testing/faulting.rs:240)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (cat of both seed files; grep of `Just(Violation` in faults.rs shows only `UnexpectedQuery` and `UncontainedSupply`; `git log -S` for both hashes returns ddc9a2d8; `git show ddc9a2d8:...faults.rs` has `let violation = Violation::UnexpectedQuery;` with `0usize..=15` and no `violation` parameter, so both lines were orphaned at birth)
- Seen by: blind-spots ([30]); refutation: confirmed; history: no rationale found for lines 7-8; the "minted during mutation review" annotations (lines 11-15, stats.txt:7) are deliberate and recorded as a practice across three commits (7626543e, f099dcb1, 50c8b0a3)
- Owner-gated: yes: removing or rewording seed lines is governed by the AGENTS.md rule to commit every seed and never strip one

Line 7 names `violation = UnaskedReply`, which `arb_connected_violation` cannot draw and `Faulting` cannot synthesize (`Violation::UnaskedReply | Violation::UnansweredQuery => unreachable!()`, faulting.rs:240); line 8 names `server_steps = 16`, outside `0..=15`, in a two-parameter shape no current test has. Proptest replays every seed in the file for every test in the file, so the lines stay live, but each now reproduces a case other than the one its comment describes, and the regression each was minted for is unreachable by the strategy. A seed is an instrument of record whose comment tells a reader what it reproduces; a stale comment turns "verified" into "told" (Principle 8). The history pass could not locate the test that produced line 8.

Evidence:

         7	cc ea434720ac87c1b9710680e38f956817be9512b9dca9d1a30aaf3d98fb5667fe # shrinks to violation = UnaskedReply, server_steps = 0, client_steps = 0
         8	cc f7515199a92a2d04d20b953c417021f4771ca93cc0ec4603cf51d6da120fdb8b # shrinks to server_steps = 16, client_steps = 0

Resolution: Owner call: (a) keep the lines and correct the comments to what the seeds now generate, or (b) remove the two lines in a commit whose message names the orphaning. The "minted ... during mutation review" notes state what the seed pins and are recorded precedent; leave them unless the owner wants the campaign reference dropped. Acceptance: every `# shrinks to` comment in the file names a value the file's current strategies can generate.

### streaming-tests-21: Inline qualified paths and split import groups where an import would do
- Where: src/tree/mirror/streaming/tests/fixtures.rs:17-18 (related: fixtures.rs:170, 181, 201, 223; capacity.rs:158; faults.rs:24, 165; local_eq.rs:299-301; skeleton.rs:149; stats.rs:22, 33)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for the qualified spellings over the partition, excluding `use` lines: exactly these sites)
- Seen by: structure-prose ([17]), api-economics ([48]); refutation: confirmed, with one site withdrawn (tests.rs:232's function-local `use materialized::{Error, Violation}` keeps an unaliased `Error` out of a file that imports `mirror::Error as MirrorError`); history: no rationale found (c6fe4018 is recorded precedent for importing trait names and is also the commit that left `use serde::Serialize;` as a lone group)
- Owner-gated: no

`std::collections::BTreeSet` is spelled inline four times in fixtures.rs and once in capacity.rs; `crate::tree::Root` inline at faults.rs:24, 165 and local_eq.rs:299-301 while both files import other `crate::tree` items; skeleton.rs:149 calls `crate::tree::mirror::streaming::message::initiates(` inline while stats.rs imports it; stats.rs imports `super::fixtures` in two statements (22, 33); fixtures.rs:17 isolates `use serde::Serialize;` with no blank line before the following doc comment. None of these qualifications disambiguates anything.

Evidence:

        17	use serde::Serialize;
        18	/// A 32-byte path with the given prefix, zero-padded.
        19	pub(super) fn path_at(prefix: &[u8]) -> Path {

    local_eq.rs:
       298	    fn check(
       299	        local: crate::tree::Root,
       300	        remote_0: crate::tree::Root,
       301	        remote_1: crate::tree::Root,

Resolution: Add the imports at file top, merge the split groups, and put a blank line after fixtures.rs:17. Acceptance: the grep matches only `use` lines; each file imports `super::fixtures` once.

### streaming-tests-22: fixtures.rs hand-rolls what its own `grown` and `rooted` express; the ceiling-of-node expression is written three times
- Where: src/tree/mirror/streaming/tests/fixtures.rs:336-380 (related: fixtures.rs:61-96, 424-471; src/tree/arb.rs:509-522; local_eq.rs:278-282; wedge.rs:99-100)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read: the two version-chain loops in `one_sided_pair` (342-351, 360-369) and `divergent_cells_pair` (431-440, 446-459) are one tick per path with `Action::Insert(Message::new(()))`, exactly `grown(node, party, 1, &(), &paths)`; the `root` closures at 372-379 and 463-470 are `rooted` verbatim; `ceiling_of` repeats the expression; `arb.rs:517 root_with_ceiling` has the same body as `rooted_at`, and `arb.rs:509 leaf_sibling_path` is private)
- Seen by: structure-prose ([8]), api-economics ([44]); refutation: confirmed; history: no rationale found (the older builders date from ddc9a2d8; `grown`/`rooted` arrived a week later in 97dfbcdf and the older ones were never folded)
- Owner-gated: no

`one_sided_pair` and `divergent_cells_pair` each contain two loops that are `grown` and a `root` closure that is `rooted`; `ceiling_of` is `rooted`'s ceiling expression a third time; `rooted_at` and `arb::root_with_ceiling` are one constructor under two names; and the "leaf whose path differs only in byte 31" builder exists as `arb::leaf_sibling_path`, `local_eq::leaf`, and inline at wedge.rs:99-100. The fixture file's contract is "a shared base grown per party", and it should read that way once.

Evidence:

       372	    let root = |node: Option<TreeNode<height::Root>>| Root {
       373	        ceiling: node
       374	            .as_ref()
       375	            .map(TreeNode::ceiling)
       376	            .cloned()
       377	            .unwrap_or_default(),
       378	        root: node,
       379	    };
       380	    (root(a_node), root(b_node))

Resolution: Rewrite `one_sided_pair` and `divergent_cells_pair` as `grown` calls over path lists (parties 0/1 and 0/2/1 as today) and `rooted`; define `rooted(node)` as `rooted_at(node, ceiling_of(&node))`; make `leaf_sibling_path` `pub(crate)` in `tree::arb` (or move it to fixtures) and use it at local_eq.rs:278 and wedge.rs:99; drop one of `root_with_ceiling`/`rooted_at` (the arb.rs side is outside this partition and optional). Acceptance: fixtures.rs contains one `Version::tick` loop (inside `grown`) and one `map(TreeNode::ceiling)` expression; `bytes[31] =` appears once in the streaming test tree.

### streaming-tests-23: A family-shaped test hand-drives `TestRunner` (no shrinking, no seed) around a counter that cannot fail
- Where: src/tree/mirror/streaming/tests/local_eq.rs:144-254 (related: local_eq.rs:24-25, 127-143)
- Class / severity / confidence: test-quality / medium / high
- Provenance: assessed (read the loop body: every iteration either panics on one of its five asserts or executes `nondegenerate += 1`; the role probe `remote_plus(&[0])` is computable inside a `proptest!` closure); the history pass verified via `git log -S'nondegenerate += 1'` that the counter has been unconditional since 97dfbcdf
- Seen by: structure-prose ([5]), blind-spots ([28]), api-economics ([43]); refutation: confirmed and added that the `ValueTree` and `TestRunner` imports at 24-25 exist only for this loop; history: no rationale found
- Owner-gated: no

`free_insertions_are_invisible_to_the_local_view` samples 32 `arb_divergence()` cases through `TestRunner::deterministic()` inside a plain `#[test]`, so a failing case neither shrinks nor persists to `local_eq.txt`, and the same 32 specs run forever. `assert_eq!(nondegenerate, CASES)` can never be the failing assertion, and the doc's "counted" and "at 100% frequency" framing describes that vestigial counter rather than a property. AGENTS.md: when the claim is a family, state it as a proptest invariant so the shrunk counterexample rides along as a committed seed.

Evidence:

       145	fn free_insertions_are_invisible_to_the_local_view() {
       146	    const CASES: u32 = 32;
       147	    let mut runner = TestRunner::deterministic();
       148	    let strategy = arb_divergence();
       149	    let mut nondegenerate = 0u32;

       247	        nondegenerate += 1;
       248	    }
       249	
       250	    assert_eq!(
       251	        nondegenerate, CASES,
       252	        "every constructed case realizes a nondegenerate LocalEq pair"
       253	    );

Resolution: Move the body into the file's `proptest!` block as `fn free_insertions_are_invisible_to_the_local_view(spec in arb_divergence())` with `prop_assert!`s (`#![proptest_config(ProptestConfig::with_cases(32))]` if the case count matters), delete `CASES`, `runner`, `nondegenerate`, and the two imports at 24-25, and retitle the doc from "NONDEGENERACY, counted" to the invariant. Acceptance: no `TestRunner` in local_eq.rs; the test is a `proptest!` member; a forced failure writes to `proptest-regressions/tree/mirror/streaming/tests/local_eq.txt`.

### streaming-tests-24: The role election and the advertised length are mirrored twice in the suite
- Where: src/tree/mirror/streaming/tests/skeleton.rs:148-168 (related: stats.rs:63-75; src/tree/mirror/streaming/backend.rs:385-394)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read: `advertised_len` (163-168) is byte-for-byte the body of `pub(crate) fn len` on `Root<B>` (backend.rs:389-394); `initiates` is called at skeleton.rs:149 and stats.rs:74, each wrapped in a role mirror)
- Seen by: structure-prose ([9]), api-economics ([47]); refutation: reframed ([47]'s premise that private fields force the duplication is wrong: `tree::Root`'s fields are visible to descendants of `tree`, which is how skeleton.rs reads `root.root` and fixtures.rs constructs `Root { ceiling, root }`; the `live()` detour through `StreamingRoot<Local>` is a choice); history: no rationale found (`client_role` was re-rigged in 872aaaa1 as "the test mirror" of `message::initiates`; `a_initiates` and `live` were added the next day without reference to it)
- Owner-gated: no (an optional `tree::Root::len()`/`ceiling()` accessor would be an API decision; the in-suite consolidation needs none)

`skeleton::client_role` (returning `Party`) and `stats::a_initiates` (returning `bool`) both mirror `message::initiates` over two `tree::Root`s; `skeleton::advertised_len` reimplements `Root<B>::len`, which `stats::live` reaches by cloning into `StreamingRoot<Local>`. Two election mirrors can drift from each other and from `initiates` independently, and the inline `crate::tree::mirror::streaming::message::initiates(` path at 149 hides that stats.rs imports the same function.

Evidence:

       148	pub(super) fn client_role(client: &TreeRoot, server: &TreeRoot) -> Party {
       149	    if crate::tree::mirror::streaming::message::initiates(
       150	        advertised_len(client),
       151	        &client.ceiling,
       152	        advertised_len(server),
       153	        &server.ceiling,
       154	    ) {

       163	fn advertised_len(root: &TreeRoot) -> u64 {
       164	    root.root
       165	        .as_ref()
       166	        .map(|node| node.len() as u64)
       167	        .unwrap_or_default()
       168	}

Resolution: Keep one `client_role(&Root, &Root) -> Party` (in skeleton.rs or fixtures.rs) importing `initiates`, compute lengths through `Root<Local>::len` or one shared `advertised_len`, and define `a_initiates` as `client_role(a, b) == Party::I` (or use `client_role` directly) and `live` as the shared length helper. Acceptance: one call to `initiates` and one definition of the advertised length remain in the partition.

### streaming-tests-25: skeleton.rs re-derives height parity beside its own `asks`, and keys channel projections by `&'static str`
- Where: src/tree/mirror/streaming/tests/skeleton.rs:351-365 (related: skeleton.rs:133-138, 453-455, 649-668; src/tree/mirror/streaming/materialized/progress.rs:11-13)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read; `progress::Kind` derives `Clone, Debug, Eq, PartialEq` only, progress.rs:12, which is why `trace_channels` cannot key by it)
- Seen by: structure-prose ([20]); refutation: confirmed; history: no rationale found (`asks` and the closures were written in the same commit)
- Owner-gated: no

`decode` defines `asker_at`/`answerer_at` with `h.is_multiple_of(2)` although `asks(p, height)` (133-138) states the same law, and the comment at 453-455 spells it a third time; `trace_channels` keys its map by a hand-picked string per `EventKind` variant because `Kind` carries `pending` data and derives no `Ord`. One statement of a law per module; an enum over a string discriminant (types-first).

Evidence:

       351	    // The endpoint the height-parity theorem assigns each role at height `h`.
       352	    let asker_at = |h: usize| {
       353	        if h.is_multiple_of(2) {
       354	            initiator
       355	        } else {
       356	            responder
       357	        }
       358	    };

Resolution: `let asker_at = |h| if asks(Party::I, h) { initiator } else { responder };` and `answerer_at` as its complement; introduce a small `#[derive(Ord)] enum Channel` (pending erased) for `trace_channels` keys, or a `Kind::channel()` discriminant method. Acceptance: `is_multiple_of` appears only inside `asks`; `trace_channels`' key type contains no `&'static str`.

### streaming-tests-26: The whole-subtree shed count is never pinned above one
- Where: src/tree/mirror/streaming/tests/stats.rs:195-218 (related: src/tree/mirror/streaming/materialized/unknown.rs:104, 150; src/tree/mirror/streaming/stats.rs:92-98; tests/session_stats.rs:109, 302-335)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of every `messages_shed` assertion in src/ and tests/: `== 0`, `== 1`, or the conservation identity, at stats.rs:185, 186, 208, 210, 258, 259 and session_stats.rs:64, 66, 109, 111, 324, 328; grep of every `shed(` call: `unknown.rs:94` sheds 1 for a leaf, `:104` and `:150` shed `node.len() as u64` for a wholly-known internal node; read `sessions_conserve_the_live_count` (session_stats.rs:302-335): its strategy draws `send_all` only, never a redaction)
- Seen by: blind-spots ([22]); refutation: confirmed (adds that the gate's `mutants-list` leg is list-only, so a surviving mutant here is not gated either); history: no rationale found (487e17ea and 7626543e describe where sheds are counted and pin "one shed"; the open investigation brief `.agent-notes/2026-08-21-unknown-pruning-survivor/README.md:92-94` proposes exactly this instrument)
- Owner-gated: no

`SessionStats::messages_shed` documents that a subtree pruned without descending "adds its exact live-leaf count", and `unknown.rs` implements that with `stats.shed(node.len() as u64)` at two sites. Every committed pin asserts exactly 1 or 0, and the only conservation proptest never redacts, so the `len`-crediting arm is exercised only with `len == 1` and its multi-leaf semantics is unobserved. A mutant `node.len() as u64` -> `1` at either site survives both the walk-tier and the public suites (Principle 6: a criterion the wrong implementation also passes is decoration).

Evidence:

       208	    assert_eq!(a_stats.messages_shed, 1);
       209	    assert_eq!(a_stats.messages_gained, 1);
       210	    assert_eq!(b_stats.messages_shed, 0);
       211	    assert_eq!(b_stats.messages_gained, 0);

    unknown.rs:
       103	            Dominance::After => {
       104	                stats.shed(node.len() as u64);
       105	                return Ok(None);
       106	            }

Resolution: Add a walk-tier pin with `grown` and `act`: a shared base of k >= 2 leaves under one controlled child (paths `[0x30, i, 0..]`) plus one shared leaf elsewhere so the root stays disputed; side b is the base plus one extra on party 1; side a is the base with the k leaves forgotten on party 2 (a lacks child 0x30 and its ceiling dominates the k versions). Assert `b_stats.messages_shed == k` and `live(&theirs) == b_before - k + b_stats.messages_gained`. Add a redaction step to `sessions_conserve_the_live_count`'s strategy (a drawn subset of the shared sends) so the law is checked with shed > 1 over the wire. Acceptance: a committed test asserts `messages_shed >= 2` from one whole-subtree prune; the public conservation proptest draws redactions; the `node.len()` -> `1` mutant at unknown.rs:104 and :150 is caught.

Construction: Build the pair as described with `fixtures::grown` and `traverse::act(node, [(path, version, Action::Forget)], ..)` and run `mirror_with_stats(a, b)`: today's code reports `messages_shed == k` on b; replacing `node.len() as u64` with `1` at unknown.rs:104 makes the new assertion fail while every existing test stays green.

### streaming-tests-27: The wedge.rs module doc cites seed-file paths that do not exist, inside a decision-history paragraph
- Where: src/tree/mirror/streaming/tests/wedge.rs:12-18 (related: formal/lean/StreamingMirror/Mux/Instances.lean:49-55 (outside the partition) repeats the narrative)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git ls-files | grep proptest-regressions` lists `proptest-regressions/pairwise.txt` and `proptest-regressions/shadow_validity.txt` and no `tests/*.proptest-regressions`; `ls tests/*.proptest-regressions` matches nothing; `git ls-tree 97dfbcdf tests/` shows both cited paths existed when the doc was written)
- Seen by: structure-prose ([2]), blind-spots ([29]), api-economics ([38]); refutation: confirmed; history: deliberate-but-expired (the paths were real at 97dfbcdf; ce3664dd, 2026-09-01, centralized every sibling seed under `proptest-regressions/`, and its sweep list did not include wedge.rs; the paragraph's presence was deliberately reworded in 2921784c, so deleting the whole paragraph revisits an LLM-authored prose choice, not an owner ruling)
- Owner-gated: no

The paragraph names `tests/pairwise.proptest-regressions` and `tests/shadow_validity.proptest-regressions`. Neither is tracked; the seeds live at `proptest-regressions/pairwise.txt` and `proptest-regressions/shadow_validity.txt`, and AGENTS.md says `tests/seed_liveness.rs` fails exactly that sibling layout, so the doc names a layout the gate forbids, written the same day the guidance landed. The rest of the paragraph is a design-history argument ("This bridge therefore constructs...") rather than a statement of what the module is.

Evidence:

        12	//! On the committed seeds: `tests/pairwise.proptest-regressions` and
        13	//! `tests/shadow_validity.proptest-regressions` are integration-level
        14	//! seeds (three-peer networks, version-addressed leaves, whole-`Rumors`
        15	//! action lists) that realize the wedge's *jam mechanism*, not its
        16	//! byte-exact shape; a structural equality pin needs hand-placed paths.
        17	//! This bridge therefore constructs the pair deterministically and pins
        18	//! the decoded skeleton to the literal.

Resolution: Replace 12-18 with one present-tense sentence without paths: "The pair is hand-placed: a structural-equality pin needs exact paths, which content-addressed generators cannot supply." If the seed observation must survive, it belongs in `.agent-notes/`. Instances.lean:49-55 carries the same narrative and should be trimmed in the same pass (outside this partition). Acceptance: `grep -rn 'proptest-regressions' src/tree/mirror/streaming/tests/wedge.rs` is empty.

### streaming-tests-28: The Lean wedge literal is transcribed by hand with no mechanical comparison
- Where: src/tree/mirror/streaming/tests/wedge.rs:38-40 (related: wedge.rs:41-65; formal/lean/StreamingMirror/Mux/Instances.lean:56-67; justfile:287-292)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read `Instances.lean:56-67` against `wedge.rs:48-64` scope for scope: they match today, including `fan := 7` and `capLevel := 1` against `LEAN_WEDGE_FAN` and `LEAN_WEDGE_CAP_LEVEL`; read justfile:289-292: `citecheck` resolves citation names against the `crates/before` test inventory only, and no gate leg reads Instances.lean)
- Seen by: structure-prose (open question), blind-spots ([33]); refutation: confirmed; history: deliberate-and-holds (the manual discipline is stated in code and dates from 97dfbcdf; nothing records why hand-checking suffices)
- Owner-gated: yes: adding a gate leg is gate policy; declaring the manual discipline acceptable is a documented design decision

`lean_wedge_literal()` mirrors `Mux.wedge` and the doc asks the human to keep them in sync. The Rust pin (`wedge(6) == lean_wedge_literal()`) checks the generator against the transcription, not the transcription against the Lean, so a change to `Instances.lean`'s literal (the shape `wc_impossibility` quantifies over) would leave the bridge asserting the wrong shape with the gate green. Doctrine: any quantity computable two ways gets a committed test comparing them.

Evidence:

        38	/// The Lean wedge literal, transcribed scope-for-scope from the Lean
        39	/// definition `Mux.wedge` — the Lean definition is the source of truth;
        40	/// if the literal changes there, change this.

Resolution: Owner choice: (a) a small `tools/` check that extracts `def wedge` from `Instances.lean` and compares it to a serialized `lean_wedge_literal()` (the `muxprobe-expected.tsv` precedent suggests a Lean-emitted expectation file a Rust test could read), wired into the gate; or (b) record at the site that the transcription is human-checked and why that is acceptable. Acceptance: a gate leg fails when the two diverge, or the doc states the accepted manual discipline.

## Positives

- Every test in the partition carries a doc comment, and most state their invariant precisely enough to check body against doc line by line (verified: I read all nine files; the two inaccuracies found are streaming-tests-5 and -13).
- The oracles are independent and derived rather than hand-tallied: `join_oracle` (tests.rs:152-161) is the single differential for content, stated with its justification and reused by `capacity.rs`; `stats.rs` derives its expected dispute counts as the prefix closure of an antichain of cells split by depth parity, cross-checks the oracle's own arithmetic (`by_depth == [1, 2, 4]`, sum 7), and adds the conservation law `live_after == live_before + gained - shed`.
- `capacity_stress_witness_requires_inter_level_fan` (capacity.rs:171-195) is an adequacy demonstration of the kind the doctrine asks for: it shows the return queue needs the fan (stalls at 253, completes at 254, `high_water >= 254`) rather than asserting that an oversized constant works; `capacity_stress_covers_every_queue_role` pairs every capacity ceiling with liveness floors (`channels > 0`, `sends > 0`, `receives > 0`, `blocked_send_polls > 0`, more than one typed height observed).
- `skeleton::decode` (335-479) does not merely rebuild the skeleton; it audits every publication against the model's count and parity laws as a total check over the trace, naming the offending scope, and `assemble` enforces the Lean `wellFormed` conjuncts on the way in. `skeleton::announced` reconstructs the skeleton from the payload-erased reply transcript with no tree access, an oracle that shares no code with the walk.
- The Lean mirrors are faithful and their deviations are recorded at the site: `asks`, `viewEnc` (tokens 2/3/4, the R child emitted only to the asker), `LocalEq` minus the vacuous `fan`/`capLevel` conjuncts (skeleton.rs:21-27), the `wedge` literal (matched scope for scope against `Instances.lean:56-67` today), and `Skel::max_fan` reflecting both `wellFormed` bounds; every cited Lean name exists under `formal/lean` (the lenses verified by grep; I confirmed `def wedge`).
- The fault matrix in `faults.rs` runs every injected violation and every greeting lie in both driver orientations and asserts error routing by side; `materialized_backend_failures_are_fail_fast` pins the exact failing operation identity through sibling cancellation; the join oracle and the redaction fixture run in both argument orientations, exercising the error-flip path of `descend` as well as the direct one.
- Determinism is used uniformly: every session runs under `run_to_quiescence` on the test thread, thread-local instruments scope cleanly through nested `with_*` closures, every convergence harness calls `trace.assert_valid()`, and grep finds no sleeps, runtimes, `#[ignore]`, `TODO`, debug prints, dead feature gates, or BLAKE3/alternating-protocol identifiers in code (verified).
- `fixtures.rs` states the invariants that make the oracles exact (disjoint parties keep extras concurrent; slot columns never collide; both remotes advertise a joined ceiling and equal set sizes so the local role is provably identical across the two sessions), and `view_projection_is_sound` asserts that premise rather than assuming it; `wedge_trees` asserts its own election precondition with an actionable message.
- The committed seeds carry provenance annotations ("minted against a deliberately neutralized containment check during mutation review; it passes on real code"), making the adequacy discipline visible in the artifact.

## Open questions for Finch

- Seed disposition (streaming-tests-20, and the orphaning that streaming-tests-17's enumeration would cause for faults.txt lines 7, 8, 12, 15): the AGENTS.md rule says never strip a seed. Recommendation: remove lines 7-8 in a commit whose message names the orphaning (they never matched a live strategy), and when the rosters are enumerated, retire the seed lines that name enumerated parameters in the same way, leaving the `materialized_backend_failures_are_fail_fast` seeds (lines 9-10) in place.
- The Lean wedge literal (streaming-tests-28): a mechanical comparison or an accepted manual discipline? Recommendation: a Lean-emitted expectation file (the `muxprobe-expected.tsv` precedent) read by `wedge_generator_matches_the_lean_literal`, so the Rust literal can be deleted rather than maintained.
- Every local-walk-versus-`Tree::join` check in `src/` runs at `WindowConfig::FLOOR`; wider windows are exercised by the proxy harness and the integration window suites. Does any wide-window run compare against `Tree::join` rather than pairwise convergence alone? Recommendation: one wide-window arm in `streaming_matches_join_oracle`, since a positional-pairing bug that manifests only with several scopes in flight has no differential oracle today (outside this partition to confirm).
- No test in this partition drops the `mirror` future mid-session. The backend contract documents cancel safety for `Leaf::leaf`; where is the walk's behavior under cancellation pinned? Recommendation: if nowhere, a walk-tier pin that cancels at a drawn poll count and checks the local root is unchanged.
- The em-dash convention (streaming-tests-4) is codebase-wide (roughly 116 `//` sites under src/) and sourced from the global doctrine rather than AGENTS.md. Recommendation: a one-line `tools/` lint wired into the gate, plus one sweep, rather than partition-by-partition fixes.
- `tree::Root::len()` and `ceiling()` accessors (`pub(crate)`) for the three test sites that re-derive them: recommendation is no; the in-suite consolidation in streaming-tests-24 removes the duplication without an API change.

## Dropped

- [0]'s related site tests.rs:190-193 ("the closing request for it prunes"): ordinary English, not a retired identifier (refuted).
- [17]'s site tests.rs:232 (function-local `use materialized::{Error, Violation}`): defensible, since hoisting an unaliased `Error` into a file that imports `mirror::Error as MirrorError` would want an alias (refuted).
- [47]'s premise that `tree::Root`'s private fields force two spellings of the length and election: descendant-module visibility already lets the tests read the fields (refuted); the duplication itself survives as streaming-tests-24.
- [50]'s option to dissolve `terminal_errors_preempt_parked_peers`: it pins the crate-owned `Client`/`Server` routing that `descend`'s equal-version path relies on (refuted); relocation survives as streaming-tests-2.
- [23]'s claims that redaction below the root is "never sampled" and that the pruned-to-nothing reply is unexercised: depth-1 redaction is reached by hash collision and the root-fan corner is exercised (reframed); the depth-with-redaction gap survives as streaming-tests-7.
- [24]'s claim that depth reaches the streaming walk solely through hand fixtures: `scheduled_structured_disputes_match_oracle` draws pyramids to depth 6 and boundary fans to depth 31 against the oracle (reframed); the missing deep-spine arm survives as streaming-tests-6.
- [26]/[41]'s coverage-risk framing for the sampled rosters: the per-run miss probability is about 2% for one of 64 cells and negligible for the lie cells (reframed); enumeration survives as streaming-tests-17 on determinism, legibility, and cost.
- [18] (long lines in `proptest!` bodies): folded into streaming-tests-3, whose helper is the fix.
- [51] ("knobs", "genuine"): folded into streaming-tests-8 as dialect sites.
- Duplicates merged: [29], [38] into streaming-tests-27; [39] into streaming-tests-5; [31], [37] into streaming-tests-8; [27] item 1 and [36] into streaming-tests-18; [27] items 2-3 and [46] into streaming-tests-14; [27] item 4, [28], [43] into streaming-tests-23; [7], [40] into streaming-tests-3; [44] into streaming-tests-22; [47] into streaming-tests-24; [34], [45] into streaming-tests-12; [42] into streaming-tests-16; [49] into streaming-tests-9; [48] into streaming-tests-21.
- The blind-spots open question on `Faulting`'s radix-0xff assumption (faulting.rs:230-232): outside the partition; the debug check belongs in faulting.rs.
- The refutation pass's new observations 3 (driver.rs inline `mod tests`) and 4 (roster tags in transcript.rs and progress.rs): outside the partition; referenced from streaming-tests-2 and -8 so one sweep covers them.
