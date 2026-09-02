# Partition tests-resource-link-window: Integration tests: allocation meters, message size, async wire, latency, opening supply, routed and TCP links, window suites

## Partition summary

This partition is the crate's instrument bench. Two allocator meters (`tests/decode_alloc.rs`, `tests/encode_alloc.rs`) price the wire codec's declared-length reads and frame writes in bytes and allocation events through `rumors::testing` entries. `tests/target_message_size.rs` captures the wire to prove the run-batching setting reaches the greeting and binds both encoders. `tests/async_wire.rs` holds concurrent `Rumors::gossip` to a union oracle. Three conformance drivers run concrete transports through the public `rumors::conformance::link::check`: the bench harness's delayed pipe (`tests/latency_link.rs`), the per-session TCP link (`tests/tcp_link.rs`), and the routed adapter over sockets and the in-memory network (`tests/routed_link.rs`, which adds a pooling dialer and a three-node mesh beside a stalled header). `tests/opening_supply.rs` pins question ownership on the wire. The window family (`window_census`, `window_corners`, `window_knee`, `window_operator`, `window_sweep`, and the ignore-gated `tradeoff_probe`) holds the sync-memory-budget derivation against measured node residency and exact virtual-time hop counts read off `benches/support/latency.rs`'s paused-clock wire, which seven of these binaries compile in by `#[path]`.

I read all fourteen files in full with line numbers, 3,111 lines, every one of them test code; I also read the src and tests/common sites each finding cites, the bench support module, `.config/nextest.toml`, the justfile and CI recipes, and the refutation pass's run log. I ran no cargo, just, or test command myself; the one test run cited below was performed by the refutation pass and I read its log.

The quality is high where it matters most. Every test carries a doc comment, and in every file the body asserts what the doc states, with one exception discussed below. The meters pair ceilings with liveness floors and calibrate their own harness overhead; the capture suites carry negative controls that prove their comparators can see a difference; the knee suite places each measured cell against the derivation for its own session shape and asserts the landing; the sweep suite pins the liveness of a generator dimension. The dominant defects are structural and prosaic. One is a genuine verification gap: `window_census`'s headline admittance test runs both arms of its differencing at the identical all-ones window, because its "tight" budget sits below the derivation's flat pre-charge, so its central assertion is `X <= X + admitted` today. The rest are duplication (ten copies of one bootstrap fixture, three copies of one capture parser, two copies of the allocator scaffolding) and hand-transcribed numbers that drifted from the constants they restate (`28 + m` where the crate ships 43; a nextest comment describing a wall-clock bound removed five weeks ago).

## Findings

### tests-resource-link-window-1: nextest profile comment describes a wall-clock upper bound window_corners no longer asserts
- Where: .config/nextest.toml:32-35 (related: tests/window_corners.rs:206-212)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git show f0762b8b -- tests/window_corners.rs` removes `catch_up < Duration::from_secs(5)` and adds `/// Both legs assert floors only`; `git log -1 -- .config/nextest.toml` gives a4d2e546, 2026-07-24, before f0762b8b, 2026-07-31; `grep -n 'from_secs' tests/window_corners.rs` finds no wall-clock ceiling)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired
- Owner-gated: no

The profile comment justifies running the window suites without scheduling isolation by pointing at one load-sensitive assertion with 8x headroom. That assertion was removed by f0762b8b, whose message states the stronger argument ("a threshold over a noisy quantity relocates the flakiness to the threshold"); the config comment now describes a bound that does not exist and leaves the correct rationale, that every wall-clock assertion is a floor, unstated. This breaches the no-ghost-references rule.

Evidence:

    32	# of the suite like any other test. The one wall-clock upper bound among
    33	# them (window_corners' real-clock promptness cross-check) holds a
    34	# ~0.5 s measured catch-up under a 5 s budget: that ~8x headroom is what
    35	# absorbs load there, where the virtual pins need none.

    206	/// Both legs assert floors only, because only the never-undercharge
    207	/// direction is load-independent on a wall clock: machine load inflates real
    208	/// elapsed time and can never compress it, so these assertions read the
    209	/// same under any suite parallelism.

Resolution: Rewrite lines 32-35 to state what is: the window suites assert on exact virtual time, and window_corners' real-clock leg asserts lower bounds only, which load can inflate but never falsify, so none needs isolation. Drop the 0.5 s and 5 s figures. Acceptance: the comment names no upper bound and no budget figure and agrees with window_corners.rs:206-212.

### tests-resource-link-window-2: latency.rs's dead_code comment names one caller where the partition has three
- Where: benches/support/latency.rs:398-400 (related: tests/latency_link.rs:138, tests/latency_link.rs:152, tests/window_corners.rs:223, tests/window_corners.rs:236, benches/gossip_fixed.rs:56-57)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn new_wall_clock tests/ benches/` lists the five call sites; `git blame` dates the comment to 1988de6da2 and the first extra caller to ebe663fbd the same day)
- Seen by: structure-prose, blind-spots, api-economics; refutation: reframed (attribute deletion refuted); history: deliberate-but-expired
- Owner-gated: no

The comment says `new_wall_clock` is used only by `window_wallclock`; tests/latency_link.rs and tests/window_corners.rs call it twice each. Hand-maintained caller enumerations rot silently, and this one rotted the day it was written. The anchor is outside this partition's file list but is compiled verbatim into seven of its binaries, and the partition's callers are what falsify it. The api-economics lens also proposed deleting the four per-item `#[allow(dead_code)]` attributes; the refutation pass showed that would break `just clippy` (`-D warnings`), because benches/gossip_fixed.rs:56-57 includes the module with no module-level allow and uses only `DelayedWire::new`, so the attributes are load-bearing and stay.

Evidence:

    398	    // Used only by `window_wallclock`; the module is `#[path]`-included by
    399	    // several targets, each seeing its own copy's usage.
    400	    #[allow(dead_code)]

Resolution: Drop the caller-naming clause here and in the sibling comments at 463-465, 511-513, 534-536, keeping only the rationale ("the module is `#[path]`-included by several targets, each seeing only its own usage"); keep every `#[allow(dead_code)]`. Acceptance: no comment in latency.rs names an including target; `just clippy` stays clean.

### tests-resource-link-window-3: async_wire.rs restates pairwise.rs's union property under a qualifier whose referent was deleted
- Where: tests/async_wire.rs:1-59 (related: tests/pairwise.rs:32-37, tests/pairwise.rs:47-61, tests/pairwise.rs:219-237, tests/common/wire.rs:1, tests/async_wire.rs:61-81)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read both files side by side; `dup` in pairwise.rs:32-37 is `bootstrap_fork`; `git log --all -- tests/sync_wire.rs` ends at 83edcd944, 2026-07-16, and the file is absent at HEAD; `git grep asynchronous -- tests` returns async_wire.rs:1 and common/wire.rs:1)
- Seen by: api-economics (structure-prose separately noted the two identical bodies); refutation: confirmed; history: deliberate-but-expired
- Owner-gated: no

`async_gossip_converges_on_the_union` is `pairwise::gossip_unions_content` plus `pairwise::gossip_converges`'s fingerprint check, built from the same generators and helpers. The binary exists because 691909e7 split wire tests into "async and sync binaries"; the sync binary was deleted on 2026-07-16, so "the *asynchronous* gossip path" names the only path there is, a ghost reference by contrast. What remains distinct is the `T = String` variant, one property. The duplicate costs a link unit and 256 cases of two bootstraps plus a session for no new claim. If the file is kept instead, its two bodies (47-58, 69-80) differ only in the type parameter and generator and belong in one generic helper; `assert_fingerprints_equal` also spells `rumors::Rumors<T>` while `rumors::Peer` is imported.

Evidence:

     1	//! Convergence of the *asynchronous* gossip path.

    47	        let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
    48	        let a = build_local(bootstrap_fork(&seed), &a_actions);
    49	        let b = build_local(bootstrap_fork(&seed), &b_actions);
    50	
    51	        let mut expected = readout(&a.snapshot());
    52	        expected.extend(readout(&b.snapshot()));
    53	
    54	        wire_gossip(&a, &b);

Resolution: Make `pairwise::gossip_unions_content` generic over the value strategy (or add one `_string` case) so the `String` payload rides the union oracle there, delete tests/async_wire.rs, and reword tests/common/wire.rs:1 to "Wire helpers for `Rumors::gossip` over in-memory links". Acceptance: tests/async_wire.rs is gone; pairwise.rs exercises a non-primitive payload through the union oracle; `git grep -n asynchronous tests/` describes no gossip path.

### tests-resource-link-window-4: "byte-for-byte" attributed to a hash that is blind to message bytes
- Where: tests/async_wire.rs:26-27 (related: tests/async_wire.rs:39-41, src/tree.rs:20-22)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (src/tree.rs:20-22 reads "a pure function of the version set, blind to message bytes"; `git log -S'blind to message' -- src/tree.rs` gives 961f63c6, 2026-08-18, after the helper's doc was written)
- Seen by: blind-spots; refutation: confirmed; history: deliberate-but-expired
- Owner-gated: no

The helper's doc says equal hashes mean the pair "agrees byte-for-byte"; since 961f63c6 the Merkle hash is a function of the version set alone. The byte-level agreement is what the `readout` equalities above establish; the hash pins the version set. An inaccurate testdoc is a bug in the test. Moot if the file is deleted per finding 3, but the same sentence should not migrate.

Evidence:

    26	/// The converged pair agrees byte-for-byte and causally: equal observable
    27	/// hashes and equal `latest` versions.

Resolution: State what the two checks pin: equal version sets (`hash`) and equal causal frontier (`latest`); fix the echo at lines 39-41 ("byte-identical (`hash`)"). Acceptance: no doc in the file claims byte identity from the hash.

### tests-resource-link-window-5: the allocator meters duplicate their scaffolding, and decode_alloc's run_body wrapper is bypassed
- Where: tests/decode_alloc.rs:20-25 (related: tests/encode_alloc.rs:18-23, tests/decode_alloc.rs:62-69, tests/encode_alloc.rs:58-65, tests/decode_alloc.rs:92-94, tests/decode_alloc.rs:276)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both files; `grep -rn 'static METER_LOCK' tests/` returns the two sites; `grep -c 'rumors::testing::' tests/decode_alloc.rs` returns 17)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no-rationale-found (encode_alloc created today, 0521b207, copying the block)
- Owner-gated: no

The `#[global_allocator]` static, `METER_LOCK` with its doc, and `metered<T>` are identical between the two binaries. Separately, decode_alloc.rs defines `run_body(len)` as a one-line wrapper over `rumors::testing::lone_record_run` and then calls `lone_record_run` directly at line 276, so the wrapper is half-adopted; and the file fully qualifies `rumors::testing::` at seventeen call sites where one `use` line would do (imports over long qualified paths).

Evidence:

    20	#[global_allocator]
    21	static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;
    22	
    23	/// Serializes metered regions: the allocator counters are process-global,
    24	/// so concurrent tests would attribute each other's traffic.
    25	static METER_LOCK: Mutex<()> = Mutex::new(());

    92	fn run_body(len: usize) -> Vec<u8> {
    93	    rumors::testing::lone_record_run(len)
    94	}

    276	        body.extend_from_slice(&rumors::testing::lone_record_run(HONEST_LEN / 2));

Resolution: Either merge the two meters into one `alloc_meter.rs` binary (one allocator, one lock, one `metered`), or extract a `tests/support/meter.rs` holding `ALLOCATOR`, `METER_LOCK`, and `metered`, `#[path]`-included by both (the idiom the window suites use for latency.rs; a `#[global_allocator]` cannot live in tests/common because every binary including `common` would inherit it). In decode_alloc.rs, use `run_body` at line 276 or delete the wrapper, and import the `rumors::testing` functions once. Acceptance: `grep -rn 'static METER_LOCK' tests/` returns one site; decode_alloc.rs has no fully qualified `rumors::testing::` call and one spelling of the run-body constructor.

### tests-resource-link-window-6: default-dialect vocabulary across the partition: "honest" (including two constant names), "genuinely", "knob", "seam", "law", "reds"
- Where: tests/decode_alloc.rs:31-41 (related: tests/decode_alloc.rs:157, tests/decode_alloc.rs:269, tests/encode_alloc.rs:4, tests/target_message_size.rs:4, tests/target_message_size.rs:154, tests/target_message_size.rs:321, tests/target_message_size.rs:344, tests/target_message_size.rs:351, tests/tcp_link.rs:9, tests/routed_link.rs:10, tests/routed_link.rs:17, tests/routed_link.rs:258, tests/window_corners.rs:1, tests/window_corners.rs:86, tests/window_corners.rs:202, tests/window_knee.rs:32, tests/window_operator.rs:9, tests/window_operator.rs:18)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep over the fourteen files: "honest" on 37 lines, "genuinely" on 16, "knob" at five target_message_size sites, "seam" at routed_link.rs:10 and 258, "law" at routed_link.rs:17 and window_operator.rs:9, "reds" at decode_alloc.rs:157 and 269)
- Seen by: structure-prose; refutation: confirmed; history: contradicts-hard-rule (the owner's writing standard lists these substitutions; the standard postdates most sites; "honest" appears on 143 src lines, so the sweep is tree-wide)
- Owner-gated: no

The brief's prose standard names these tells: moralized code ("honest", "genuine"), metaphors promoted to jargon ("knob", "seam", "law"), and coinages ("reds" as a verb). In decode_alloc the word is doubly awkward: under the model of record every peer is honest, so an "honest frame" conflates the trust term with what is meant, a fully delivered well-formed one. Every sentence competes with the contract the reader came for.

Evidence:

    31	/// The payload length of the honest fully-delivered frames the liveness
    32	/// floors meter.
    33	const HONEST_LEN: usize = 8 * 1024 * 1024;
    34	
    35	/// A non-power-of-two honest payload length.

    157	/// passes the zero-delivered ceiling and the floors, and reds here.

Resolution: Rename `HONEST_LEN`/`HONEST_ODD_LEN` to `FULL_LEN`/`FULL_ODD_LEN` and write "fully delivered" where "honest" appears in the meter docs; delete "genuinely" wherever the sentence survives without it (it does at every site here); "the knob" becomes "the setting" or the method name; "address seam" becomes "the `Addr` boundary"; "never-block law" becomes "the router's never-blocks guarantee"; "reds here" becomes "fails here"; window_corners.rs:1 "Boundary honesty of the sync-budget interface" becomes "The sync-budget interface at its boundaries". This partition's sites are listed above; the tree-wide sweep is a separate pass. Acceptance: `grep -n -i 'honest\|genuine\|knob\|seam\|\breds\b'` over the partition returns nothing, or only uses the owner rules acceptable.

### tests-resource-link-window-7: decode_alloc testdoc claims an admitted-overhang complement no meter in the file exercises
- Where: tests/decode_alloc.rs:263-271 (related: src/testing.rs:152-156, src/tree/mirror/streaming/remote/codec/budget.rs:102, src/tree/mirror/streaming/remote/codec/budget.rs:128-132, src/tree/mirror/streaming/remote/codec/decode/tests.rs:965-989)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (src/testing.rs:155 `decode_supply_frame_budgeted(read, usize::MAX)`; budget.rs:130 saturates at `MAX_RUN_BUDGET_BYTES`, `u32::MAX as usize - SUPPLY_FRAME_OVERHEAD`, so an 8 MiB lone record is within budget; the overhang admission is pinned only by `oversized_lone_record_still_decodes`, unmetered)
- Seen by: blind-spots; refutation: confirmed; history: no-rationale-found (the sentence was inaccurate when written, ed0f1775e)
- Owner-gated: no

The doc says the lone-record tests above pin "that the admitted overhang still reaches the body read", but those tests run through `decode_supply_frame`, which decodes at the saturated ceiling budget, so their record is never an overhang. The ingress gate decides one boundary (one record over budget admitted, two rejected), and the meter prices only the rejection side. An inaccurate testdoc is a bug in the test.

Evidence:

    269	/// identically but request the body's bytes, and reds here. The honest
    270	/// lone-record tests above are the complement, pinning that the admitted
    271	/// overhang still reaches the body read.

Resolution: Run `supply_full_delivery_costs_at_most_payload_plus_chunk` (or a twin) through `decode_supply_frame_budgeted(&bytes[..], 0)`, making the lone record a true overhang at no extra cost, and reword lines 269-271 to name that test as the complement. Acceptance: a metered test decodes a lone record under a budget smaller than the record, asserts `Ok`, and holds the same `[N, N + chunk]` band.

### tests-resource-link-window-8: a public-vocabulary matchability pin with an unfailable assert lives in the allocator meter
- Where: tests/decode_alloc.rs:317-334 (related: tests/decode_alloc.rs:1-12, tests/decode_alloc.rs:17)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`git log -L 317,334:tests/decode_alloc.rs` shows the block introduced by 39290c4a "export HeadError"; the body constructs and matches the same variant with no `metered` call)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no-rationale-found (placement unargued; the file already imported `rumors::error`)
- Owner-gated: no

`leaf_run_head_defect_is_publicly_matchable` meters nothing and its `matches!` cannot fail at runtime; its value is that the file compiles against `rumors::error::HeadError`. That is a public-API surface pin, which the module doc ("Allocator metering for the wire decoders'...") does not admit. A reader auditing the error taxonomy will not look here, and modules have one responsibility.

Evidence:

    317	/// The head-grammar defect carried by [`LeafRunError::Head`] is public
    318	/// vocabulary: a caller outside the crate can write the type
    319	/// `rumors::error::HeadError` and match the variant a non-shortest-form
    320	/// head classifies as.
    321	#[test]
    322	fn leaf_run_head_defect_is_publicly_matchable() {

Resolution: Move the test to the binary that pins public error vocabulary (or a small `tests/error_surface.rs`), state in its doc that compiling is the check (a `let _: HeadError = ...;` suffices), and drop the now-unused `HeadError`/`LeafRunError` imports here. Acceptance: decode_alloc.rs contains only metered tests; the pin lives beside its peers.

### tests-resource-link-window-9: two module-doc claims are false today: latency_link's plural "protocols" and tcp_link's "one Link instantiation over real sockets"
- Where: tests/latency_link.rs:3-6 (related: tests/tcp_link.rs:3-4, tests/routed_link.rs:1-12, src/protocol.rs:15-19, src/link/routed/endpoint.rs:20-25, tests/common/tcp.rs:18-21)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (src/protocol.rs:15-19 has `V2` as the sole variant; routed_link.rs runs `RoutedLink<TcpDial>` over loopback sockets through the same conformance suite; tcp.rs:18-21 already names the distinguishing property)
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (latency_link.rs:5 predates the V1 retirement; tcp_link.rs:3 predates routed_link)
- Owner-gated: no

Both sentences were true when written and neither was swept by the commit that falsified it. "the protocols" is V1-era residue; "the workspace's one Link instantiation over real sockets" is a uniqueness claim routed_link.rs contradicts. Prose states what is.

Evidence:

     3	//! `benches/support/latency.rs` builds the delayed-pipe link the latency
     4	//! benchmarks sweep; these tests run it through the public
     5	//! [`rumors::conformance::link`] suite so the sweep measures the protocols, not
     6	//! an accidentally nonconforming transport. Delays live in virtual time

    tcp_link.rs:
     3	//! `common::tcp` is the workspace's one [`Link`](rumors::link::Link)
     4	//! instantiation over real sockets, and `tests/disruption.rs` trusts it

Resolution: latency_link.rs:5: "measures the protocol". tcp_link.rs:3-4: "the simulations' direct, one-connection-per-stream Link over real sockets" (the property tcp.rs:18-21 explains). Acceptance: both sentences are true of today's tree.

### tests-resource-link-window-10: paused-clock conformance runs lack the timeout the suite's own docs require
- Where: tests/latency_link.rs:28-43 (related: src/conformance/link.rs:24-26, tests/tcp_link.rs:54-61, tests/routed_link.rs:94-101, .config/nextest.toml:26)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (src/conformance/link.rs:24-26 reads "**Run under a timeout**: the contract's liveness clauses fail as hangs"; `grep -n 'Duration::\|tokio::time' src/conformance/link.rs` is empty, so the suite arms no timer of its own; tcp_link and routed_link wrap their calls, these two do not)
- Seen by: blind-spots; refutation: confirmed; history: no-rationale-found (a3da46c42c wrote the requirement and re-pointed these tests without adding one)
- Owner-gated: no

A wedged clause here is reported only by nextest's 180 s terminate. Under `start_paused`, `tokio::time::timeout` is the ideal detector: with no runnable task and no shorter wire timer pending, auto-advance jumps to the deadline and the clause fails in zero wall time with the harness's own message.

Evidence:

    28	#[tokio::test(start_paused = true)]
    29	async fn conforms_at_zero_delay() {
    30	    rumors::conformance::link::check(async || latency::delayed_pair(CAPACITY, Duration::ZERO))
    31	        .await;
    32	}

Resolution: Wrap both `check` calls in `tokio::time::timeout(SUITE_TIMEOUT, ...).await.expect("conformance suite ran past its liveness bound")` as the socket runners do, with a `SUITE_TIMEOUT` constant. Acceptance: both tests carry the timeout; a deliberately wedged pipe fails with that message rather than after 180 s.
Construction: In a scratch copy of `latency::delayed_pair`, make one stream's `poll_read` return `Pending` without arranging a wake; today the test hangs until nextest kills it, with the timeout it fails immediately.

### tests-resource-link-window-11: opening_supply shadows the imported path_radix with an identical closure; over-generic seeded<T>; "These tests" for one test
- Where: tests/opening_supply.rs:103-106 (related: tests/opening_supply.rs:18, tests/opening_supply.rs:21, tests/opening_supply.rs:25-28, tests/opening_supply.rs:8, tests/opening_supply.rs:114, tests/common/shape.rs:17-24, tests/hop_trace.rs:535-544)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (shape.rs:22-24 defines `path_radix` as `leaf_path(version)[0]` with `leaf_path` the SHA3-256 digest; line 21 imports it and lines 85 and 94 use it; `seeded` has one call site at `T = u64`, line 80; the module has one `#[test]`)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired (the closure predates the shared helper by two hours and survived the SHA3 sweep 4f18c347 as a duplicate hash site)
- Owner-gated: no

The fixture imports `common::shape::path_radix` for staging and then rebinds the same name to a closure computing the same formula for its self-checks, pulling in `use sha3::Digest;` for that alone; the comment frames it as an independent check but it is the same derivation, so a reader must verify two definitions agree for no gain. `seeded<T>` carries a five-bound generic signature for one `u64` call; the module doc says "These tests pin" over one test. hop_trace.rs:535-544 (another partition) carries the same closure.

Evidence:

    103	    // Fixture self-checks: one shared radix, disputed; the subtree holder
    104	    // is the smaller set and initiates. A leaf's path is the full-width
    105	    // SHA3-256 hash of its version's canonical bytes.
    106	    let path_radix = |version: &Version| sha3::Sha3_256::digest(version.as_bytes())[0];

Resolution: Delete the closure and the `sha3::Digest` import, using the imported `path_radix` in the self-checks (they remain valid checks that staging landed the shape); drop the `radix` rebinding at 114; make `seeded` return `Rumors<u64>`; write "This test pins" or split the module doc's two claims into two tests. Acceptance: one `path_radix` in scope, no `sha3` import, `seeded() -> Rumors<u64>`.

### tests-resource-link-window-12: two absence assertions have no in-file positive control
- Where: tests/opening_supply.rs:143-156 (related: tests/gossip_snapshot.rs:305, tests/gossip_snapshot.rs:382, src/tree/mirror/streaming/remote/codec/capture.rs:197-206, src/tree/mirror/streaming/remote/codec/signal.rs:196)
- Class / severity / confidence: test-quality / nit / medium
- Provenance: verified (`grep -rn 'Initiator stream 0 (height 31)' tests/*.rs` finds the positive uses only in gossip_snapshot.rs; the format string is capture.rs:199)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`frames_labeled(&capture, "QueryEmpty") == 0` and `!capture.contains("Initiator stream 0 (height 31)")` both pass vacuously if the renderer relabels (the `Query(` == 1 sibling proves the parser alive, not the `QueryEmpty` label). The gap is mitigated across binaries (gossip_snapshot.rs searches the header positively and the `.snap` files carry both labels), so this is a nit; but nothing in this file distinguishes "absent" from "renamed".

Evidence:

    153	    assert!(
    154	        !capture.contains("Initiator stream 0 (height 31)"),
    155	        "an empty early set opens no opening-supply stream"
    156	    );

Resolution: Assert a same-format line exists in the capture (any line starting `Responder stream ` or `Initiator stream `), or share the header and label strings as constants from tests/common so a relabel fails to compile here. Acceptance: renaming the census header at capture.rs:199 fails this test on its own.

### tests-resource-link-window-13: routed_link repeats the establishment tail three times and the timeout wrapper five times; vestigial braces around send_all
- Where: tests/routed_link.rs:76-90 (related: tests/routed_link.rs:53-68, tests/routed_link.rs:94-101, tests/routed_link.rs:104-128, tests/routed_link.rs:169-192, tests/routed_link.rs:198-216, tests/routed_link.rs:238-243, tests/routed_link.rs:259-278, tests/hop_trace.rs:550-552)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read the three constructors and five `timeout(` sites; `git log -L238,243:tests/routed_link.rs` lists b2675dbe, ce27df86 "rework Batch into a closure scope", 6e4b6eea, 212c6914 "send_all")
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired (the braces scoped a `Batch` guard, then a closure, then nothing)
- Owner-gated: no

`tcp_pair`, `memory_pair`, and `pooled_tcp_pair` end in the same nine lines (`tokio::join!(b.link(addr), a_incoming.accept())`, two expects, the `dialer_first` swap); `pooled_endpoint` re-spells `tcp_endpoint` for a different dial type; `tcp_conformance` names the `timeout(SUITE_TIMEOUT, check(..)).await.expect(..)` wrapper and four later tests inline it anyway. The two bare blocks at 238-243 are husks of a removed `Batch` scope. Maintenance surface with no information content.

Evidence:

    82	    let (linked, arrival) = tokio::join!(b.link(a_addr), a_incoming.accept());
    83	    let dialed = linked.expect("establishment succeeds");
    84	    let (_info, accepted) = arrival.expect("the router delivers the link");
    85	    if dialer_first {
    86	        (dialed, accepted)
    87	    } else {
    88	        (accepted, dialed)
    89	    }

    238	        {
    239	            seed.send_all(0..48u64).unwrap();
    240	        }
    241	        {
    242	            newcomer.send_all(48..96u64).unwrap();
    243	        }

Resolution: Write one generic `endpoint<D: Dial>` and one `establish<D>(b, a_incoming, addr, dialer_first)`, and one `conformance_under(pair)` wrapper used by all eight conformance tests; drop the braces (and consider naming 48/96 so line 249's `96` is derived). hop_trace.rs:550-552 carries the same braces. Acceptance: one spelling of the establishment tail and one of the timeout wrapper; no single-statement bare blocks in the file.

### tests-resource-link-window-14: target_message_size: near-duplicate pair builders, a thrice-spelled corpus seed, an unenforced OFF_DEFAULT guard, and "budget" for "target"
- Where: tests/target_message_size.rs:26-47 (related: tests/target_message_size.rs:79-106, tests/target_message_size.rs:40, tests/target_message_size.rs:97, tests/target_message_size.rs:275, tests/target_message_size.rs:160, tests/target_message_size.rs:304-307, src/tree/mirror/streaming/window.rs:275)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both builders; `0x5eed_0f1e_a55e_d000` at lines 40, 97, 275 and `seed_from_u64(0)` at 85, 272; no inequality assertion on `OFF_DEFAULT_BUDGET`; window.rs:275 makes the "512 MiB" accurate today)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`diverged_pair` and `seeded_diverged_pair` differ only in `Peer::seed()` versus `seed_rng(..0)` and in interleaved versus sequential sends; the network identifier is not part of a version or path, so the two convergence tests lose nothing by using the seeded builder. The corpus seed is a magic number spelled three times. `OFF_DEFAULT_BUDGET`'s doc argues the pin cannot pass by coincidence, but nothing asserts `OFF_DEFAULT_BUDGET != DEFAULT_SYNC_MEMORY_BUDGET`, and "512 MiB" is a hand-maintained restatement. Line 160's "If the budget silently regressed" names the wrong setting in a test about the target.

Evidence:

    26	fn diverged_pair(left_target: usize, right_target: usize) -> (Rumors<u64>, Rumors<u64>) {
    27	    block_on(async {
    28	        let left = Peer::seed()
    29	            .sync_window_floor()
    30	            .target_message_size(left_target)
    31	            .into_rumors();

    304	/// A budget far from [`rumors::DEFAULT_SYNC_MEMORY_BUDGET`]'s 512 MiB, so
    305	/// the wire-equality pin below cannot pass by the two configurations
    306	/// coinciding.
    307	const OFF_DEFAULT_BUDGET: usize = 1024 * 1024;

Resolution: Delete `diverged_pair`; route `zero_target_still_converges` and `mixed_targets_interoperate` through `seeded_diverged_pair(l, r, (DIVERGENT_PER_SIDE, DIVERGENT_PER_SIDE))`; name `const CORPUS_SEED: u64` and `const IDENTITY_SEED: u64`; add `const _: () = assert!(OFF_DEFAULT_BUDGET != DEFAULT_SYNC_MEMORY_BUDGET);` and drop the literal from the doc; write "target" at line 160. Acceptance: one pair builder; each seed literal once; a compile-time guard on the off-default budget.

### tests-resource-link-window-15: three binaries each own a parser for the rendered capture's signal line
- Where: tests/target_message_size.rs:122-130 (related: tests/target_message_size.rs:108-120, tests/target_message_size.rs:132-152, tests/opening_supply.rs:42-61, tests/gossip_snapshot.rs:161-178, tests/gossip_snapshot.rs:215-229, src/tree/mirror/streaming/remote/codec/capture.rs:173-212)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'split_once(" / ")' tests/` returns exactly opening_supply.rs:53, gossip_snapshot.rs:170, target_message_size.rs:126; the renderer is one function)
- Seen by: structure-prose, api-economics; refutation: confirmed (the three differ subtly: `is_supply_signal` never strips the trailing ` /`, `signal_semantic` additionally requires an uppercase first character); history: no-rationale-found (today's 3327a92b6 re-documented two of them separately)
- Owner-gated: no

`is_supply_signal`, `frames_labeled`, and `signal_semantic` all parse the renderer's `<state code> / <Semantic> /` line with the same bare-digit test, each with its own paragraph explaining why the digits disambiguate, and each restating the oracle caveat about relabeling. A renderer-vocabulary change (a sanctioned re-accept class) must be chased into three parsers; the maintenance cost has already been paid once.

Evidence:

    125	fn is_supply_signal(line: &str) -> bool {
    126	    let Some((code, rest)) = line.trim_start().split_once(" / ") else {
    127	        return false;
    128	    };
    129	    !code.is_empty() && code.bytes().all(|b| b.is_ascii_digit()) && rest.starts_with("Supply")
    130	}

Resolution: Move `signal_semantic` (the most general form) into tests/common/gossip_snapshot.rs as `pub fn`, add `pub fn frames_labeled(capture, prefix) -> usize` and a per-direction variant on top of it, and express all three suites' counts through them; keep the oracle-assumption paragraph once at the shared definition. Acceptance: `grep -rn 'split_once(" / ")' tests/` returns one site under tests/common.

### tests-resource-link-window-16: the one nonzero binding run target is never checked for convergence
- Where: tests/target_message_size.rs:224-267 (related: tests/target_message_size.rs:61-74, tests/target_message_size.rs:1-8, tests/common/gossip_snapshot.rs:476-481, tests/common/gossip_snapshot.rs:492-516)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`capture_gossip` returns only the rendered string and the drivers `expect` Ok; the `frames` closure never inspects the handles; convergence is asserted only via `diverged_pair(0, 0)` and `diverged_pair(0, DEFAULT_TARGET_MESSAGE_SIZE)`, both at exchanged minimum 0; `grep -rn 'target_message_size('` outside the file finds only target 0 and builder unit tests)
- Seen by: blind-spots; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The module doc claims "any minimum (including the degenerate zero) leaves reconciliation convergent"; the body asserts convergence at exchanged minimum 0 only. `nonzero_minimum_binds_both_encoders` runs four cells at `SMALL_TARGET`, the one target in the suite that splits runs, and compares frame counts without looking at the resulting sets. A run-splitting bug that dropped or duplicated a record at a small nonzero target would complete `Ok`, keep every tuple equality, and pass. `capture_gossip_returning` exists for exactly this.

Evidence:

    225	    let frames = |left, right| {
    226	        let (l, r) = seeded_diverged_pair(
    227	            left,
    228	            right,
    229	            (BINDING_MESSAGES_PER_SIDE, BINDING_MESSAGES_PER_SIDE),
    230	        );
    231	        directional_supply_frames(&capture_gossip(l, r))
    232	    };

Resolution: Use `capture_gossip_returning` in the `frames` closure and assert `l.snapshot() == r.snapshot()` per cell, so each of the four cells also pins convergence. Acceptance: each cell asserts snapshot equality, and the assertion is reachable (an encoder that skips the last record of a split run fails it).
Construction: In a scratch build, make the supply encoder drop the final record of any run it splits under a nonzero target; today every assertion in this test still passes (the frame counts stay equal across the mixed and uniform-small cells); with snapshot equality per cell it fails.

### tests-resource-link-window-17: tradeoff_probe: import spacing, undocumented helpers, a bare expect("small"), a hand-maintained 5,431
- Where: tests/tradeoff_probe.rs:44-46 (related: tests/tradeoff_probe.rs:56, tests/tradeoff_probe.rs:63, tests/tradeoff_probe.rs:127, tests/tradeoff_probe.rs:173, src/tree/mirror/streaming/window.rs:265)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the lines; window.rs:265 `SCOPE_ENVELOPE_BYTES: usize = 5_431` makes the prose accurate today; `git show c6fe4018` inserted the `use serde` pair with no separator in 17 test files)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no-rationale-found (mechanical-edit residue)
- Owner-gated: no

Two `use serde` items sit against the first doc comment with no blank line (the same shape recurs in six other test files, mostly outside this partition); `diverged` and `run_cells` are the partition's only helpers without doc comments; `u32::try_from(transfer).expect("small")` is not the one-line proof the crate asks of expect messages and sits beside `transfer as f64` casts of the same quantity; line 56 transcribes `SCOPE_ENVELOPE_BYTES` as "5,431 B" while lines 137-139 deliberately refuse to transcribe the intercept. Moot if the file is retired per finding 18.

Evidence:

    44	use serde::Serialize;
    45	use serde::de::DeserializeOwned;
    46	/// One-way delay for the virtual-time measurements (the timer grain).

    173	            hops as f64 <= (exact * f64::from(u32::try_from(transfer).expect("small"))).ceil(),

Resolution: Merge the imports into the main block and add the blank line; doc-comment `diverged` and `run_cells`; have `wire_hops` return `u32` through `hops_on_lattice` (finding 26) so `f64::from` applies uniformly; drop "= 5,431 B" and let the name carry it. Acceptance: rustfmt-clean import block; every fn documented; no `expect("small")`; no literal 5,431 in prose.

### tests-resource-link-window-18: tradeoff_probe's ignore gate is a recorded ruling whose rationale lives only in history, and nothing schedules the run that public rustdoc quotes
- Where: tests/tradeoff_probe.rs:186-188 (related: tests/tradeoff_probe.rs:1-10, src/peer.rs:414-417, justfile:772-775, .github/workflows/ci.yml:134, .github/workflows/ci.yml:180-181, .agent-notes/2026-07-22-sync-budget/sync-budget.md:267-276)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`grep -n 'tradeoff\|run-ignored\|--ignored' justfile` matches only the `window-tradeoff` table recipe; ci.yml has no `--run-ignored`; Cargo.toml has no `[[test]]` entry; src/peer.rs:414-417 quotes "ran 1.35–1.96× the form's figure (`tests/tradeoff_probe.rs`)"; `git show e48bf5ab` records "Kept ignore-gated: a full cell run is ~11 s in release and several times that in the debug gate, so promotion into the regular suite is declined")
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed (strengthened by the peer.rs citation); history: deliberate-and-holds (the ignore gate is Finch's ruling; a recipe is not precluded by it)
- Owner-gated: yes: whether to add a gate-adjacent recipe or retire the instrument is a verification-policy decision

The ruling that keeps this instrument out of the regular suite (cost) still holds, but the module doc says only "runs only by explicit request" without saying why, so the next reader re-derives or reopens it. Separately, nothing in the justfile or CI invokes the run; it compiles under `test-all` but its assertion of record never executes, so it can sit red indefinitely while `Peer::sync_memory_budget`'s public rustdoc quotes its measurement. Its distinct claim, the wave form at the design corpus (62,500 a side) and the design record (m = 172), is covered by nothing else (`window_operator` runs 8,192 u64 records). An instrument no recipe invokes is decoration (Principle 2), and the justfile is the source of truth for verification (AGENTS.md).

Evidence:

    186	#[test]
    187	#[ignore = "one-shot validation instrument: run explicitly with --run-ignored"]
    188	fn tradeoff_closed_form_validation_run() {

    src/peer.rs:
    414	    /// Measured: sessions whose serialized one-way trips are counted
    415	    /// exactly on a virtual clock, at 8–26 MB budgets on the minimal
    416	    /// and design corpora, ran 1.35–1.96× the form's figure
    417	    /// (`tests/tradeoff_probe.rs`).

Resolution: In either outcome, state the cost rationale in the module doc at the `#[ignore]`. Then, owner's call: (a) add a `just tradeoff-probe` recipe (`cargo nextest run --release --test tradeoff_probe --run-ignored all`) to the CI `instruments` job beside `worst-cases-pin`, with the module doc pointing at the recipe instead of spelling the cargo command; or (b) retire the file, moving the design-corpus and design-record cell into an enforced suite if that claim is wanted, and re-denominating or excising the peer.rs:414-417 citation. Acceptance: the module doc names the cost; and either `grep -n tradeoff_probe justfile .github/workflows/*.yml` finds the recipe and step, or the file is gone and peer.rs cites no unrun instrument.

### tests-resource-link-window-19: window_census rests correctness on the runner where its sibling meter closes the same hazard with a lock; the peak differencing is spelled twice
- Where: tests/window_census.rs:15-16 (related: tests/window_census.rs:87-102, tests/window_census.rs:273-286, src/testing.rs:51-52, src/tree/typed/untyped.rs:53-56, tests/decode_alloc.rs:23-25, src/conformance/backend/tests.rs:37)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (the counters are `static AtomicUsize` at untyped.rs:54-56; testing.rs:51-52 states the premise; `grep -c 'cargo test' justfile` is 0 and `grep -c 'cargo nextest' justfile` is 8; decode_alloc.rs:23-25 and conformance/backend/tests.rs:37 serialize the same hazard with a static mutex)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: deliberate-and-holds (premise stated at both the suite and the accessor; the gate is nextest-only; R12's fix d8afb610 chose a lock for the sibling genre)
- Owner-gated: no

The premise is stated and the gate enforces it, so this is a robustness improvement rather than a defect: under `cargo test` (threads in one process) the two census tests would reset and read each other's peaks, and the two version-bound tests allocate nodes concurrently. decode_alloc.rs faces the identical process-global-counter problem and closes it runner-independently with `METER_LOCK` ("the suite is correct under any test runner's threading"). "Sound" is also used loosely here. Separately, `overhead` (89-102) and `floor_overhead_is_bounded_by_content` (274-281) spell the same before/reset/reconcile/peak/after arithmetic.

Evidence:

    15	//! The census is process-global, which is sound here because nextest runs
    16	//! each test in its own process.

Resolution: Add `static CENSUS_LOCK: Mutex<()>` taken in every test body (the version-bound tests construct nodes too), mirroring decode_alloc's `metered`; reword the module doc to say the lock makes the suite runner-independent; hoist the differencing into `overhead` returning `(overhead, after)` and call it from the floor test. If the owner rules `cargo test` unsupported, keep the premise but drop "sound" for "correct under nextest's process-per-test model". Acceptance: every window_census test serializes on one lock (or the doc names the runner requirement as such); one site of the peak-differencing arithmetic.

### tests-resource-link-window-20: window_census's admittance test is vacuous: TIGHT_BUDGET resolves to the all-ones floor, so both differenced arms run the identical window
- Where: tests/window_census.rs:27-28 (related: tests/window_census.rs:87-102, tests/window_census.rs:116-136, tests/window_corners.rs:174, src/tree/mirror/streaming/window.rs:397-399, src/tree/mirror/streaming/window.rs:436-456, src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:647-656, src/tree/mirror/streaming/stats.rs:151)
- Class / severity / confidence: verification-gap / high / high
- Provenance: verified (the refutation pass ran `cargo nextest run -p rumors --all-features -E 'binary(window_census)' --no-capture`; I read its log at scratchpad/refute-tests-resource-link-window/window_census.log, lines 35-37: `budget 0, divergence 20000: peak 182214, generations 48002+99576, overhead 34636`, `budget 65536, divergence 20000: peak 182214, generations 48002+99576, overhead 34636`, `admittance 25154 (capacities sum 33)`; I read `from_budget` for the mechanism)
- Seen by: blind-spots, api-economics (as a missing liveness floor); refutation: reframed and raised to high on the run; history: no-rationale-found
- Owner-gated: no

A capacities sum of 33 over 33 heights means every height resolves to capacity 1 at `TIGHT_BUDGET`: `from_budget`'s `charge(k)` starts at the flat decode-fan term `supply_fans` (window.rs:397-399, 437), so when that term alone exceeds the budget, `charge(mid) <= budget` never holds, `lo` stays 1, and every non-root height is clamped to 1. The 64 KiB budget is below that pre-charge (e48bf5ab's decomposition puts the decode fans at 0.21 MB), and the pre-charge is independent of set size. The two arms of the differencing therefore run the identical configuration, the readings are identical, and `windowed <= floor + admitted` is `X <= X + 25154`. The constant's doc, "A budget that binds at test scale: a few scopes per level", is false at this session size. No floor in the suite would have caught this: the census counter's liveness is pinned at unit level (malformed.rs:647-656 asserts `residency >= SMALL`), but nothing here shows the differenced quantity alive. Meters need liveness floors (Principle 2), and the cheapest artifact that passes must be the intended one (Principle 6). The same 64 KiB budget at window_corners.rs:174 (`growth_during_a_session_only_serializes`) resolves to the floor by the same mechanism (assessed, not run), so that test's "The window derives from the sizes exchanged at the greeting" describes no size-dependent derivation in the shape exercised.

Evidence:

    27	/// A budget that binds at test scale: a few scopes per level.
    28	const TIGHT_BUDGET: usize = 64 * 1024;

    124	    let floor = overhead(0, DIVERGENT_WIDE);
    125	    let windowed = overhead(TIGHT_BUDGET, DIVERGENT_WIDE);
    126	    eprintln!(
    127	        "admittance {admitted} (capacities sum {})",
    128	        capacities.iter().sum::<usize>(),
    129	    );
    130	    assert!(
    131	        windowed <= floor + admitted,

    window.rs:
    436	        let charge = |k: u128| -> u128 {
    437	            let mut total = supply_fans;

Resolution: Measure first, then: pick a `TIGHT_BUDGET` whose derived capacities exceed one at 21,024 messages a side (window_knee's 2 MiB lands 4..=256 at ~2,300 messages; the population is larger here, so the budget likely needs to be larger); assert the fixture's own liveness, `capacities.iter().sum::<usize>() > capacities.len()`, beside the admittance computation; have `reconcile` return the two `Gossiped` values and assert the windowed arm's `stats.window_granted > 1` so the real session, not only the test-internals solve, is shown to widen; add `assert!(windowed > floor, ...)` with the measured value in the message; replace `saturating_sub` at lines 96 and 280 with `checked_sub(...).expect("peak covers both resting generations")` so a negative reading fails instead of reading 0; rewrite line 27's doc to state the property the budget must have. Re-measure window_corners.rs:174 alongside and either raise its budget past the pre-charge or restate that test's doc to what it exercises. Acceptance: the census log shows different capacities sums and different peaks for the two arms; the new floors are committed with measured values; a run at `TIGHT_BUDGET = 64 * 1024` fails the fixture's liveness assertion.

### tests-resource-link-window-21: magic numbers in the window suites' admittance constants and hop bounds, with one latent coupling
- Where: tests/window_census.rs:33-40 (related: tests/window_census.rs:51, tests/window_census.rs:119, tests/window_corners.rs:99, tests/window_corners.rs:113, tests/window_corners.rs:132-136, tests/window_corners.rs:242, tests/encode_alloc.rs:128, src/testing.rs:323-333, src/tree/mirror/streaming/window.rs:132, src/tree/mirror/streaming/window.rs:137)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read the sites; `window_capacities` returns `(0..=32)` entries; `FAN` is `pub(crate)` and `KEY_DEPTH` private, neither exposed through `rumors::testing`; encode_alloc.rs:128 spells the fan as `usize::from(u8::MAX) + 1`)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no-rationale-found (the history pass notes the accessor constraint)
- Owner-gated: no

window_census restates the fan width as `256`, the height count as `33`, and the common prefix as `1_024` at two sites that must agree (line 51 sends it, line 119 recomputes `session_len` from it): change one and the admittance is computed for the wrong session. window_corners uses bare `12`, `64`, `8 * 2 *`, and `64 * delay` as bounds where window_knee names the same kind of bound (`PIPELINED_HOPS`, `KNEE_MARGIN`) with rationale. Named constants over magic numbers; no hand-maintained counts.

Evidence:

    33	/// Handles one buffered scope can pin at most: a full fan of child
    34	/// references plus its own bookkeeping.
    35	const HANDLES_PER_SCOPE: usize = 256 + 2;
    36	
    37	/// Handles the assembly fan queues can hold beyond the window: one full
    38	/// fan per active level (their capacity is a correctness floor the window
    39	/// never scales; see the window module docs).
    40	const ASSEMBLY_FAN_HANDLES: usize = 33 * 256;

Resolution: In window_census: `const COMMON: usize = 1_024;` used at both sites; derive the height count from `capacities.len()` at the use site; derive the fan from `usize::from(u8::MAX) + 1` as encode_alloc does, or expose `FAN` through `rumors::testing`. In window_corners: name `LADDER_HOPS_BOUND` (12), `WAVE_FLOOR_HOPS` (64), and the linear-cost factor, carrying the rationale the inline comments at 93-97 already give. Acceptance: no bare `1_024`, `256`, `33`, or repeated `12`/`64` outside a named constant in the two files.

### tests-resource-link-window-22: ten binaries hand-roll the budgeted bootstrap fork that tests/common owns, losing its stall detector and drain check; binding_capacity is duplicated with a magic slice
- Where: tests/window_census.rs:48-75 (related: tests/window_census.rs:158-170, tests/window_census.rs:191-203, tests/window_corners.rs:33-65, tests/window_knee.rs:98-113, tests/window_knee.rs:189-216, tests/window_operator.rs:57-84, tests/window_operator.rs:100-108, tests/tradeoff_probe.rs:63-91, tests/latency_link.rs:51-71, tests/gossip_pipelining.rs:65-94, tests/hop_trace.rs:475-497, benches/window_wallclock.rs:64-89, benches/support/wire.rs:39-56, tests/common/wire.rs:34-42, tests/common/wire.rs:232-264, tests/common/window.rs:34-43)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (`grep -rln '"bootstrap newcomer"' tests/ benches/` lists ten files; `grep -rn '^fn send_random'` lists seven definitions; `grep -rln 'support/latency.rs'` lists nine includes; `grep -rn '28\.\.=30' tests/` returns window_knee.rs:101 and window_operator.rs:103; `grep -c '^mod common;'` is 0 for every measurement suite except window_sweep; common/wire.rs:40-42 `block_on` is `run_to_quiescence` and `bootstrap_fork_configured` ends with `assert_control_drained` at 262; common/window.rs:39 maps `WindowChoice::Budget` to `sync_memory_budget`)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no-rationale-found (no commit mentions `mod common`, compile weight, or link units; the direct shared fixture arrived in 919132dc after the suites, but a workable path existed since cb69fc951 and target_message_size.rs used it the same week)
- Owner-gated: no

The same block (`memory_with_capacity`, `tokio::join!(gossip, bootstrap().join)`, two `expect`s, `.sync_memory_budget(b).into_rumors()`) appears in window_census (three times), window_corners, window_knee, window_operator, tradeoff_probe, latency_link, gossip_pipelining, hop_trace, and both bench harnesses; a three-line `send_random` in five of them; the `#[path]` latency include in nine. `common::wire::bootstrap_fork_with_window_async(parent, WindowChoice::Budget(b))` already performs this fork, and does two things the copies do not: it runs under `run_to_quiescence`, so a stall in fixture construction fails at its source rather than parking `pollster::block_on` until nextest's 180 s kill, and it asserts the control drain. The suites whose whole point is exact wire measurement build their fixtures on the one poller that cannot report a stall. `binding_capacity` (min over `capacities[28..=30]`) is duplicated between window_knee and window_operator, each with its own "depths two through four; heights 30 down to 28" paragraph and the collision-expectation rationale stated only in the knee; the range is a magic slice in both. Drift between copies is what produced the stale numbers in finding 29.

Evidence:

    53	    let right = pollster::block_on(async {
    54	        let (mut provider, mut newcomer) = rumors::link::memory_with_capacity(LINK_CAPACITY);
    55	        let (served, joined) = tokio::join!(
    56	            left.gossip(&mut provider),
    57	            Peer::<u64>::bootstrap().join(&mut newcomer),
    58	        );
    59	        served.expect("serve bootstrap");
    60	        joined
    61	            .expect("bootstrap newcomer")
    62	            .expect("provider is established")
    63	            .sync_memory_budget(budget)
    64	            .into_rumors()
    65	    });

    window_operator.rs:
    103	    capacities[28..=30]
    104	        .iter()
    105	        .copied()
    106	        .min()
    107	        .expect("three engaged heights")

Resolution: Add `mod common;` to the measurement suites and build one measurement fixture (tests/common/measure.rs) on `bootstrap_fork_with_window_async`: `diverged(budget, common, left_extra, right_extra, seed) -> (Rumors<u64>, Rumors<u64>)`, `send_random`, `const ENGAGED_HEIGHTS: RangeInclusive<usize> = 28..=30` with the collision-expectation rationale once, and `binding_capacity(session_len, budget)`. The one design point to settle first is link capacity: the copies use 8 MiB pipes and `common::wire` uses `LINK_BUF` (8 KiB); since the fork is corpus construction, not the measured session, a capacity parameter on the shared helper (or accepting 8 KiB) both work. Replace every copy, the bench harnesses included. Acceptance: `grep -rn '"bootstrap newcomer"' tests/ benches/` returns one site; `grep -rn '^fn send_random'` returns one; `grep -rn '28\.\.=30' tests/` returns one; every measurement fixture runs through `run_to_quiescence` and `assert_control_drained`.

### tests-resource-link-window-23: em-dashes in seven non-doc comments
- Where: tests/window_census.rs:221-221 (related: tests/window_census.rs:235, tests/window_corners.rs:96, tests/window_corners.rs:218, tests/window_knee.rs:286, tests/window_knee.rs:315, tests/window_knee.rs:316)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -n -E '^\s*//[^/!].*—'` over the fourteen files returns exactly these seven lines)
- Seen by: structure-prose; refutation: confirmed; history: contradicts-hard-rule (the owner's doctrine: colons or semicolons over em-dashes in comments; 116 `//` comments in src carry one, so this is a partial sweep)
- Owner-gated: no

The house rule prefers colons or semicolons in `//` comments (terminal compatibility); rendered rustdoc may use em-dashes sparingly. Seven `//` comments in the window suites use one.

Evidence:

    221	    // party intervals stay shallow and stamps stay small — the many

Resolution: Replace with a colon, semicolon, or sentence break at each site; note that the tree-wide sweep (116 src sites) is separate. Acceptance: the grep above returns nothing for the partition.

### tests-resource-link-window-24: measured hop counts recorded in comments beside headroom bounds that no assertion enforces
- Where: tests/window_corners.rs:93-97 (related: tests/window_knee.rs:41-46, tests/window_operator.rs:199-201, tests/gossip_pipelining.rs:33-41, tests/hop_trace.rs:504, tests/hop_trace.rs:516)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (read the four comment sites; hop_trace.rs pins `assert_eq!(trace.hops(), 7, ...)` exactly at 504 and 516; `git show -s 814f07ad` records the figures' origin; whether they still hold is unverified, no runs)
- Seen by: api-economics; refutation: confirmed; history: deliberate-and-holds (bands with headroom are 814f07ad1's stated design: headroom admits a deeper ladder without flaking)
- Owner-gated: yes: pinning exact counts adopts snapshot discipline for these suites

The bands are deliberate and their rationale is inline (window_knee.rs:43-45). The recorded measurements beside them ("measures 8 exact hops", "2 hops over a 97-hop transfer", "7 exact hops") are hand-maintained numbers nothing enforces, so they drift without failing anything; the counts are exact and load-independent by the harness's own argument, so they are pinnable, and hop_trace.rs already pins its shapes exactly.

Evidence:

    93	    // Ladder hops: the dispute chain prunes within a few levels (the one
    94	    // shared message's subtree thins to exactly that leaf and matches), and
    95	    // the supply is one unidirectional stream. The shape measures 8
    96	    // exact hops; a size-priced session would cost ~2 hops per message —
    97	    // three orders of magnitude past this bound.

Resolution: Owner taste: either pin the exact counts (re-accepted on deliberate protocol change, as hop_trace does) or keep the bands and state only the band's rationale without the current measurement, leaving the `eprintln!` as the record. Acceptance: no comment states a current measured hop count that no assertion enforces.

### tests-resource-link-window-25: the catch-up hop ceilings have no floor
- Where: tests/window_corners.rs:98-102 (related: tests/window_corners.rs:112-115, tests/window_corners.rs:228-231, tests/latency_link.rs:104-108)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (both tests assert only `measured <= 12`; the real-clock leg at 228-231 asserts `catch_up >= 2 * delay` and latency_link.rs:104-108 asserts `hops >= 2` for a diverged pair, so the irreducible-exchange floor exists in the file on the other clock and in the sibling binary)
- Seen by: blind-spots; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

A degenerate session (one that never opened the descent, or a harness change reporting 0) passes both asymmetric catch-up tests. Pair every ceiling with a positive floor derived from the mechanism's irreducible work (Principle 2): completion depends on a reply to a delivered message, so at least two causally chained hops.

Evidence:

    98	    assert!(
    99	        measured <= 12,
   100	        "a one-common-message catch-up must cost ladder hops, not waves: \
   101	         {measured} hops",
   102	    );

Resolution: Add `assert!(measured >= 2, ...)` at both sites, stating the floor as the one request/response the transfer cannot avoid. Acceptance: both tests carry a floor; a `hops` returning 0 fails them.

### tests-resource-link-window-26: zero_budget_serializes_but_completes builds and runs its slowest pair twice because session_hops discards the reconciled handles; tradeoff_probe copies the primitive with a rounding reader
- Where: tests/window_corners.rs:128-129 (related: tests/window_corners.rs:139-140, tests/window_corners.rs:31-41, benches/support/latency.rs:515-522, benches/support/latency.rs:538-553, tests/tradeoff_probe.rs:100-125, tests/tradeoff_probe.rs:124, tests/tradeoff_probe.rs:173)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (window_corners.rs:128-129 call `pair(0, 2_048, divergence, divergence)` twice; latency.rs:520 reads `let (_pair, elapsed) = wire.round_trip_virtual(a, b);`; tradeoff_probe.rs:124 reads `(elapsed.as_millis() / DELAY.as_millis()) as u64`; `pair`'s doc at 31-32 omits `common`, and every caller passes `common >= 1`, so `common.max(1)` at line 41 is dead)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired (the second build served dd2b1645d2's two-delay differencing, retired by 814f07ad1; the probe's copy is a parallel-branch artifact predating `hops_on_lattice`)
- Owner-gated: no

The test measures a 2,048 + 2,000 + 2,000 floor session through `hops(pair(...))`, whose `session_hops` throws away the pair `round_trip_virtual` returns, then builds the identical pair again and runs a second full serialized session purely to check the outcome; the convergence check thereby certifies a different session than the one measured. tradeoff_probe works around the same signature by copying `session_hops` (100-125), replacing `hops_on_lattice`'s exactness assertion ("drift fails loudly instead of rounding silently", latency.rs:530-533) with truncating integer division and returning `u64` where every sibling returns `u32`, which is what forces the `u32::try_from(transfer).expect("small")` at line 173.

Evidence:

    128	    let measured = hops(pair(0, 2_048, divergence, divergence));
    129	    let (left, right) = pair(0, 2_048, divergence, divergence);

    latency.rs:
    519	    let mut wire = DelayedWire::new(capacity, delay);
    520	    let (_pair, elapsed) = wire.round_trip_virtual(a, b);
    521	    hops_on_lattice(elapsed, delay)

    tradeoff_probe.rs:
    124	    (elapsed.as_millis() / DELAY.as_millis()) as u64

Resolution: Have `session_hops` return `((Rumors<T>, Rumors<T>), u32)` (or a sibling that does); window_corners builds its pair once and asserts on the returned handles; tradeoff_probe collapses `wire_hops` onto it (keeping its `hash()` convergence assertion). Also give `pair`'s doc its `common` parameter and delete the dead `.max(1)`. Acceptance: one `pair(...)` call in the test; no `as_millis` division in tradeoff_probe; every hop count flows through `hops_on_lattice`.

### tests-resource-link-window-27: convergence witnessed by len() equality alone at two sites
- Where: tests/window_corners.rs:141-145 (related: tests/window_corners.rs:195-199, src/snapshot.rs:16-20, tests/routed_link.rs:250, tests/routed_link.rs:343-344, tests/tradeoff_probe.rs:119-123, src/conformance/link.rs:1061-1067)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (both sites compare `snapshot().len()`; `Snapshot` derives `PartialEq, Eq` at snapshot.rs:16; routed_link.rs:250 uses `assert_eq!(seed.snapshot(), newcomer.snapshot())` and 343-344 and tradeoff_probe.rs:119-123 compare `hash()`)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no-rationale-found (R13 ruled the same oracle shape a finding in the conformance suite and it was fixed there; window_corners was not swept)
- Owner-gated: no

Both docs say the session "converges"; the bodies prove equal cardinality, which a session that dropped one message from each side and picked up another would pass. Set equality is as cheap and strictly stronger, and it is the oracle the crate uses elsewhere. The cheapest passing artifact must be the intended one (Principle 6).

Evidence:

    141	    assert_eq!(
    142	        left.snapshot().len(),
    143	        right.snapshot().len(),
    144	        "the serialized session still converges",
    145	    );

Resolution: Compare `left.snapshot()` with `right.snapshot()` (or their `hash()`) at 141-145 and 195-199. Acceptance: no `snapshot().len()` equality stands alone as a convergence witness in the window suites.

### tests-resource-link-window-28: growth_during_a_session_only_serializes never witnesses that growth landed mid-session, and its budget resolves to the floor
- Where: tests/window_corners.rs:173-200 (related: tests/window_corners.rs:166-171, tests/window_corners.rs:174, src/peer/gossip.rs:167-171, src/peer/gossip.rs:810-814)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read; the refutation pass traced the poll structure: the racer needs 64 outer polls while a 4,000-dispute session takes far more, so most batches land after the greeting in practice; nothing asserts it; the floor-budget point follows from finding 20's mechanism at the same 64 KiB and is not run)
- Seen by: blind-spots, api-economics; refutation: confirmed at low (unwitnessed, not vacuous); history: no-rationale-found
- Owner-gated: no

The racer commits 64 batches interleaved by `yield_now`, and the test asserts both sessions complete and the follow-up converges. Nothing shows any commit landed after the session's greeting snapshot; if the interleaving front-loaded every commit, the first session already converged everything and the doc's "the next session converges whatever it missed" is exercised on nothing. The witness is cheap and deterministic under this poller. Separately, `pair(64 * 1024, ...)` sits below the derivation's flat pre-charge (finding 20), so the doc's "The window derives from the sizes exchanged at the greeting; commits racing the session make those sizes stale" describes a derivation the exercised shape never performs: the window is the floor regardless of sizes.

Evidence:

   174	    let (left, right) = pair(64 * 1024, 2_048, 2_000, 2_000);

   179	        let race = async {
   180	            for _ in 0..64 {
   181	                racer.send_all((0..32).map(|_| rng.next_u64())).unwrap();
   182	                tokio::task::yield_now().await;
   183	            }
   184	        };
   185	        let (left_result, right_result, ()) =
   186	            tokio::join!(left.gossip(&mut a), right.gossip(&mut b), race);

Resolution: Between the two sessions assert `left.snapshot().len() > right.snapshot().len()` with a message naming it as the mid-session-growth witness (or `left.snapshot().latest() != left_result.converged`, since `converged` is the merged frontier before concurrent commits are joined, gossip.rs:810-814); finish with snapshot equality per finding 27; raise the budget past the pre-charge so the size-dependent derivation the doc describes actually runs, or restate the doc. Acceptance: the witness is committed and passes; awaiting `race` before the `join!` makes it fail.
Construction: Move `race` ahead of the session (await it to completion before the `tokio::join!` of the two gossips). Every current assertion still passes, demonstrating the vacuity of the present body; with the between-sessions inequality added, the moved version fails.

### tests-resource-link-window-29: window_operator's module doc states the wire-law intercept as 28 where the crate ships 43; window_knee carries the matching stale ~36 B arithmetic
- Where: tests/window_operator.rs:3-10 (related: tests/window_knee.rs:314-317, src/tree/mirror/streaming/window.rs:221, src/peer.rs:400, src/peer.rs:455, tests/dispute_wire.rs:105, tests/dispute_wire.rs:115, tests/tradeoff_probe.rs:137-139)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git grep -n '28 + m'` finds only tests/window_operator.rs:4 and :9 in the tree outside .agent-notes; window.rs:221 `pub(crate) const DISPUTE_OVERHEAD_BYTES: usize = 43;`; peer.rs:400, 403, 431, 455 quote `(43 + m)`; `git show --stat 4dd2053c` touches peer.rs and window.rs and no tests/window_*.rs; `git blame` dates window_operator.rs:4 and :9 to 2026-07-23/24; window_knee.rs:315 dates to 90a309e0b1, 2026-07-23)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired (28 was the constant when written; it moved 28 to 34 to 35 to 43 across 2d1e6ea51, ba8045c5, 4dd2053c; e59b2292 swept the identical drift out of tradeoff_probe by reading the accessor and did not reach these two files)
- Owner-gated: no

The module doc quotes the closed form and the per-message wire law with the constant 28, twice; the crate's calibrated intercept is 43 and `Peer::sync_memory_budget`'s rustdoc says so. The test body is unaffected (it reads `window_capacities` and `supply_decode_envelope_bytes` at runtime), which is exactly why the drift went unnoticed. window_knee.rs:315 reasons from "~36 B each" (the borsh-era 28 + 8) to "~57 messages" per 2 KiB; today's minimal cell reads 43 + 9 - 1 = 51 B (`U64_ENCODED_BYTES` 9 and `MINIMAL_CELL_RESIDUAL` 1 in tests/dispute_wire.rs), about 40 messages. Hand-maintained numbers rot silently (Principle 5); the crate's own convention at tradeoff_probe.rs:137-139 ("The shipped intercept, never a transcribed copy") is the ruled pattern.

Evidence:

     3	//! `sync_memory_budget`'s docs publish the closed-form estimate,
     4	//! `slowdown(budget, m) ≈ max(1, BDP × envelope / (budget × (28 + m)))`:

     9	//! `BDP_messages = BDP / (28 + m)`, the calibrated per-message wire law
    10	//! `tests/dispute_wire.rs` pins. The `K` substitution overstates the

    window_knee.rs:
   314	    // 2 KiB in flight at 10 ms one-way models ~200 KB/s per stream: a
   315	    // BDP of ~2 KiB — ~57 messages at this corpus's measured ~36 B each
   316	    // (tests/dispute_wire.rs pins the affine per-message cost) — below
   317	    // the binding capacity, so bandwidth binds before the window does.

Resolution: At window_operator.rs:4 and :9, write the law in terms of the named constant (`DISPUTE_OVERHEAD_BYTES + m`, as window.rs:233-234 does) or as "the intercept `dispute_overhead_bytes()` exposes", and point at `Peer::sync_memory_budget` for the published form. At window_knee.rs:314-317, either excise the worked figures and keep the conclusion ("a BDP in messages below the binding capacity, per the intercept `tests/dispute_wire.rs` pins") or compute the BDP in messages from `dispute_overhead_bytes()` in code and assert it is below `capacity`. Acceptance: `grep -rn '28 + m\|~36 B' tests/` returns nothing; every wire-law figure in test prose is stated by constant name or derived from a `testing` accessor.

## Positives

- decode_alloc.rs is a model meter: every ceiling has a liveness floor (`framing_full_delivery_meters_at_least_payload`, `supply_full_delivery_meters_at_least_payload`); the non-power-of-two `HONEST_ODD_LEN` defeats the doubling-overshoot masking a power-of-two length would allow, with the reason stated at 35-41; the allocation-event ceiling catches a per-granule reservation policy whose byte reading would look identical; the typed-error assertions keep the zero-delivered ceilings from passing on an early reject, and lines 124-126 say exactly why.
- encode_alloc.rs calibrates its own harness overhead with a committed test (`harness_allocations`) so the frame constants state the writer's allocations alone, and asserts written length positive so an exact-count pin cannot pass vacuously.
- target_message_size.rs pairs every count comparison with a negative control: `sync_memory_budget_is_not_wire_visible` proves the comparator can see a difference through a wire-visible setting (337-346), and `nonzero_minimum_binds_both_encoders` states the rejected reading and carries per-direction margin self-checks (238-253) so the tuple equalities cannot hold vacuously.
- window_knee.rs couples derivation to measurement: each cell is placed by monotone search against the binding capacity of its own session shape, and the landing assertions (136-141, 162-167) fail loudly if a derivation change displaces a cell, instead of silently measuring a shape the prediction does not cover; every constant carries its rationale and headroom argument.
- window_sweep.rs pins the liveness of a generator dimension in all three ways it could rot (arm coverage, width actually granted, budget actually reaching the solve), which is rare and exactly right.
- benches/support/latency.rs argues the measurement model once and enforces it: `hops_on_lattice` refuses off-lattice drift instead of rounding, `round_trip_virtual` refuses to report a virtual figure on a running clock, and latency_link.rs pins both contracts plus run-to-run determinism, so the window suites' load-independence claim rests on committed checks.
- The conformance drivers run each transport through the public suite, in both seat orientations where construction paths differ (routed_link), under explicit timeouts, with the reason real sockets need real time stated once and well (tcp_link.rs:7-11); `pooled_mutual_sessions_converge` names the regression it pins and why the multi-thread flavor is load-bearing (218-225).
- opening_supply.rs stages its disputed-sibling shape deterministically and self-checks the landed shape before asserting on the wire, so fixture drift fails at the fixture rather than as a mysterious count.
- window_corners.rs's real-clock leg asserts only the never-undercharge direction and states why that is the load-immune one (206-212).

## Open questions for Finch

- tradeoff_probe (finding 18): wire it (a `just tradeoff-probe` recipe in the CI `instruments` job beside `worst-cases-pin`) or retire it? Recommendation: wire it. Its design-corpus and design-record cells are covered by nothing else, and `Peer::sync_memory_budget`'s public rustdoc already quotes its figure; a scheduled run makes that citation honest at the cost of ~11 s release per run.
- Shared measurement fixture (finding 22): route the window suites through `tests/common` (8 KiB `LINK_BUF`, `run_to_quiescence`, `assert_control_drained`) or through a lighter `#[path]`-included support module beside latency.rs? Recommendation: `tests/common` with a capacity parameter on the fork helper; history shows compile weight was never the recorded reason for avoiding it, and the stall detector is the point.
- Replacement `TIGHT_BUDGET` for window_census (finding 20): any value works once the fixture asserts its own liveness; the number should come from a measurement, not from me. Recommendation: pick the smallest power of two whose capacities sum exceeds 33 at 21,024 messages a side, and record the measured `windowed - floor` as the new floor's value.
- Is `cargo test` (non-nextest) a supported entry for this crate (finding 19)? If yes, the census lock is a correctness fix; if the justfile is the only sanctioned entry, it is a consistency nicety. Recommendation: add the lock either way; it costs three lines and matches the sibling meters and R12's precedent.
- Exact hop pins versus headroom bands (finding 24). Recommendation: keep the bands, drop the recorded measurements from the comments, and let the `eprintln!` lines be the record; the bands' rationale is already inline.
- Outside this partition but raised from it: `Peer::sync_window_floor` derives the same all-ones table as `sync_memory_budget(0)` (verified by reading `from_budget`); the R20 ruling asked for an explicit opt-in, not a solve-independent variant, and no record argues `Fixed` over `Budget(0)`. Recommendation: keep `Fixed` for solve-independence and say so at the declaration (peer.rs:472-479, window.rs:512-518), so the next reader does not re-derive the equivalence.
- Outside this partition, API surface the tests lean on: `Peer::seed_rng` is public but `#[doc(hidden)]` with no cfg gate (peer.rs:210-213), and `run_to_quiescence` lives behind `test-internals`, whose manifest warning ("never enable it in an application") was justified by a behavior R20's fix removed. Recommendation: document `seed_rng` with the two-universes caveat, and consider a documented home for the stall detector; both are owner decisions for the api-core partition.
- gossip_pipelining.rs (another partition) is a one-cell knee test at `DEFAULT_SYNC_MEMORY_BUDGET` with its own fixture copies and a V1 ghost at line 7 ("through both protocol implementations"). Recommendation: give window_knee's `diverged` a budget parameter and land the production-budget cell there, deleting the file and updating nextest.toml's list.

## Dropped

- [20], [40] "28 + m stale" (blind-spots, api-economics): duplicate of tests-resource-link-window-29; their arithmetic corrections (51 B, not 52; the probe's 5,431 and the 512 MiB are accurate today) are folded in.
- [25], [41] "nextest comment stale": duplicate of tests-resource-link-window-1.
- [29], [54] "tradeoff_probe unscheduled": duplicate of tests-resource-link-window-18.
- [31], [43], [51a] "fixtures duplicated" and [12] "binding_capacity duplicated": merged into tests-resource-link-window-22.
- [48] "three capture parsers" and its QueryEmpty liveness point: merged into tests-resource-link-window-15 and -12.
- [34], [53] "HeadError pin placement": duplicate of tests-resource-link-window-8.
- [51b] "merge the allocator binaries": folded into tests-resource-link-window-5 as one of two resolutions.
- [47] composite (census runner, literals, floor): split into tests-resource-link-window-19, -21, -20.
- [32], [44] "double pair build" and [28] "probe rounds hops": merged into tests-resource-link-window-26.
- [26], [45] "len-only convergence": duplicate of tests-resource-link-window-27.
- [38], [55], [51c] "routed_link braces and repetition": merged into tests-resource-link-window-13.
- [57] "target_message_size builders": duplicate of tests-resource-link-window-14.
- [35], [49] "path_radix shadow" and [36] "absence assertion": merged into tests-resource-link-window-11 and -12.
- [33] "census literals": duplicate of tests-resource-link-window-21.
- [39] "caller comment" and [46] "caller comment plus attribute deletion": comment half merged into tests-resource-link-window-2; the attribute-deletion half is refuted: benches/gossip_fixed.rs:56-57 includes latency.rs with no module-level allow and uses only `DelayedWire::new`, so the per-item allows are load-bearing under `just clippy`'s `-D warnings`.
- [58] "import spacing": duplicate of tests-resource-link-window-17.
- [52] "growth race unwitnessed": duplicate of tests-resource-link-window-28.
- [19] "async_wire bodies identical": folded into tests-resource-link-window-3 as the fallback resolution if the file is kept.
- [21]'s sub-claim "no test anywhere pins that node_census counts": refuted; src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:647-656 asserts the meter alive. The suite-local gap survives as tests-resource-link-window-20.
- [50] "sync_window_floor equals sync_memory_budget(0)": out of partition (src/peer.rs, src/tree/mirror/streaming/window.rs); recorded ruling R20 holds for the opt-in; raised as an open question.
- [59] "seed_rng doc(hidden)" and [60] "run_to_quiescence feature placement": out of partition (src/peer.rs, Cargo.toml features, src/testing.rs); feature requests, not defects; raised as open questions.
- Refutation-pass note "gossip_pipelining.rs:7 V1 ghost": out of partition; flagged in the open questions for whoever owns tests/gossip_pipelining.rs.
- [56]'s companion suggestion to pin exact counts everywhere: kept only as the owner-gated nit tests-resource-link-window-24, since 814f07ad1's band design is recorded and its rationale is inline.
