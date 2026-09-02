# Partition remote-proxy-tests: The remote proxy test suites and harness

## Partition summary

This partition is the wire proxy's own test tier: the only place in the crate where `remote::Handshaking` is driven across a `Link` with in-crate fixtures (outside it, `RemoteHandshaking` is constructed only by `streaming.rs`, `peer/gossip.rs`, and `peer/gossip/tests.rs`). `proxy/tests.rs` is the hub: it builds two materialized participants and two wire proxies over in-memory links, in an asymmetric arrangement (`reconcile`) and in the production arrangement where both local participants are the client of their own proxy (`reconcile_symmetric_accepts`), and holds the wire result to the in-process protocol (`reconcile_locally`) under channel schedules, backend fault injection, and an accept-reordering decorator. `tests/harness.rs` supplies the reusable two-proxy driver (`drive`) plus three link decorators: per-endpoint I/O adversity through `testing::wrap_link`, a one-frame mutation script (`ScriptedWrite`) that locates the state item by parsing the wire's own head grammar, and a received-greeting rewriter (`RewriteRead`); it also owns the role-election predicate `left_initiates`. The six sibling files partition the adversities: `transport.rs` (fragmentation, delay, flush buffering), `failures.rs` (typed transport faults and their stacking with backend faults), `malformed.rs` (reserved, misplaced, unordered, duplicated frames), `declarations.rs` (greeting size words the traffic does not honor), `containment.rs` (version containment over the wire), and `greeting.rs` (the greeting-carried opening listing). `start/tests.rs` pins and fuzzes the greeting ingress on the internal `receive` entry, with the reason for the internal entry stated at the top of the file; `work/tests.rs` reaches the `Work` executor's error-selection races deterministically.

I read all ten partition files with line numbers, 3380 lines in total, every one of them test code compiled under `#[cfg(test)]` into the library's unit-test binary; I also read the production sites the findings rest on (`codec/signal.rs`, `codec.rs`, `codec/greeting.rs`, `tree.rs`, `typed/node.rs`, `proxy/work.rs`, `proxy/start.rs`, `proxy/state.rs`, `streaming.rs`, `streaming/driver.rs`, `remote/streams.rs`, `link.rs`, `testing/transport.rs`, `tree/arb.rs`, `tools/testdoc`, `.config/nextest.toml`) and ran `tools/testdoc` against a scratch probe. The harness discipline is the partition's strength and it is consistent: `run_to_quiescence` wraps every session but two, so a stall is a named `Quiescence::Stalled` failure rather than a hang; every frame mutation asserts `script.fired()`; the transport-fault property predicts from a clean run whether its fault can fire and asserts `report.injected` matches that prediction; the channel-instrument test asserts every `QueueKind::PROXY` edge was exercised. Role-sensitive fixtures route through `message::initiates` rather than byte-order guesses, and the docs say why. I found no harness bug that would mask a failure.

The dominant issues are of two kinds. First, coverage shape: no frame ever crosses a link on a logical stream with index two or greater anywhere in the crate (every proxy-tier fixture is content-addressed, and every committed wire snapshot reaches stream 1 at most), so the leaf-tier decode pump and the terminal stream grammars are validated against live traffic by nothing, while ready-made deep fixtures with their oracles already exist in `tree::arb`; and the harness's default arrangement puts the right-hand proxy in the protocol Client position, which production never does, so most adversity coverage runs one endpoint through `Connect`/`CompleteConnect` impls with no production caller. Two smaller gaps follow the same pattern: the backend-fault property discards the unfaulted endpoint's tree, and the `Work::execute` accept arm is never resolved through `execute`. Second, accretion: the two-proxy topology is hand-rolled seven times, the left-is-`Server`/right-is-`Client` error projection seven times, and several small helpers, a constant, and the n-versus-m fixture two or more times each. The prose is mostly accurate and candid; the residue is five greeting-ingress test names from the retired two-frame greeting, a fuzz doc whose reach expired with the CBOR format change, and a few test docs narrower than their bodies. One gate finding falls out of the reading: `tools/testdoc` does not recognize `#[pollster::test]`, so the doc requirement is unenforced for twelve tests here and seventeen elsewhere.

Provenance for the file: most of the partition's structure comes from two commits with no argued rationale, `77674c9c` (empty body) and `83edcd94` (a work-in-progress commit), which is why most history verdicts are "no rationale found" rather than "deliberate". Where a rationale exists and holds (the zero-inversion tripwire, `cbc4a0aa`; the remote proxy's symmetric entry points, `cbfe1aff`), the findings below are reframed to what the rationale does not cover and marked owner-gated.

## Findings

### remote-proxy-tests-1: Five greeting-ingress test names speak the retired two-frame greeting
- Where: src/tree/mirror/streaming/remote/proxy/start/tests.rs:73-74 (related: start/tests.rs:92, 114, 151, 169; src/tests.rs:262-264)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git show a72fa0395:src/tree/mirror/streaming/remote/proxy/start.rs` lines 199-200 read "Send one greeting: an exactly bounded causal-version frame, then the root-fan listing frame"; `git show 4dd2053c -- start/tests.rs` renamed two sibling tests to the one-item vocabulary and left these five; the module doc at lines 3-5 today says the greeting is "one peer-controlled item")
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired at 4dd2053c
- Owner-gated: no

The names `truncated_version_header_...`, `over_declared_version_frame_...`, `empty_version_frame_...`, `trailing_version_bytes_...`, and `missing_listing_frame_...` describe a wire layout with a separate causal-version frame and root-fan listing frame that no longer exists; `missing_listing_frame_is_a_typed_read_error` cuts the last byte of the single item and involves no listing. AGENTS.md's hard rule forbids prose that refers to code or wire shapes that no longer exist, and a test name is prose a maintainer reads in `nextest` output. The same ghost survives outside the partition at src/tests.rs:262-264, whose doc says "the causal-version frame plus the root-fan listing frame" while its own body comment (272-273) says "measure its one wire item".

Evidence:

        73	#[pollster::test]
        74	async fn truncated_version_header_is_a_typed_read_error() {

        92	async fn over_declared_version_frame_is_a_typed_read_error() {
       114	async fn empty_version_frame_is_a_typed_decode_error() {
       151	async fn trailing_version_bytes_are_rejected() {
       169	async fn missing_listing_frame_is_a_typed_read_error() {

Resolution: Rename to what each test does to the one item: `truncated_item_head_is_a_typed_read_error`, `over_declared_item_length_is_a_typed_read_error`, `empty_item_is_a_typed_decode_error`, `trailing_item_bytes_are_rejected`, `cut_item_content_is_a_typed_read_error`. Sweep src/tests.rs:262-264 in the same commit. Acceptance: `grep -rn -i 'version frame\|listing frame\|version header\|version_frame\|listing_frame\|version_header' src` returns nothing.

### remote-proxy-tests-2: The random-content greeting fuzz cannot reach the map decoder its doc claims to exercise
- Where: src/tree/mirror/streaming/remote/proxy/start/tests.rs:186-194 (related: src/tree/mirror/streaming/remote/codec/greeting.rs:120-149)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read of `parse_greeting`: the first head must be `MAJOR_MAP` with value exactly `KEYS.len()`, then each key's text head must match the fixed roster byte for byte; the distribution consequence is arithmetic on those checks)
- Seen by: blind-spots; refutation: confirmed and strengthened; history: deliberate-but-expired at 4dd2053c
- Owner-gated: no

The generator feeds `content in vec(any::<u8>(), 0..96)` into an honestly sized item. `parse_greeting` accepts only a map head whose value is exactly 6 (one byte value in 256), then requires the text head and the literal bytes of `"listing"` before any version, roster, or listing decoding runs. Across 256 cases about one reaches the first key check and none reach the version atom or the listing, so the property is a heads-only no-panic check while its doc promises coverage of "key roster, version atom, listing shape and order". The doc was carried over from the pre-CBOR fuzz (`4dd2053c`), where two raw bodies fed the bit codec and borsh listing decoder directly and random bytes did reach deep. A testdoc that overstates its reach is a bug in the test (AGENTS.md, Writing tests).

Evidence:

       186	    /// The item is honestly sized around arbitrary content, so the fuzz
       187	    /// lands on the map decoder (heads, key roster, version atom, listing
       188	    /// shape and order) rather than on the allocator via a lied length —
       189	    /// the head lies are pinned deterministically above. Every outcome
       190	    /// must be `Ok` or one of the three typed greeting errors.
       191	    #[test]
       192	    fn arbitrary_greeting_bodies_never_panic(
       193	        content in vec(any::<u8>(), 0..96),
       194	    ) {

    codec/greeting.rs:
       122	    let head = cbor::read_head(&mut input).map_err(GreetingError::Head)?;
       123	    if head.major != cbor::MAJOR_MAP || head.value != KEYS.len() as u64 {

Resolution: Generate from a valid `encode_greeting` output and apply drawn mutations (byte flips at drawn offsets, truncation, insertion, drawn listing entries), or draw `Greeting` fields structurally plus a key permutation; if the pure-random arm stays, narrow its doc to the heads it reaches. Acceptance: a sampled run of the property produces at least one `Error::HandshakeListing` and at least one `GreetingError` past the first head, and the doc names only what the generator reaches.
Construction: Temporarily tally the error variant per case; under the current generator the tally is `HandshakeDecode(Shape)` for every case that is not `Ok`, with no `HandshakeListing` and no version-atom error.

### remote-proxy-tests-3: Import hygiene: a crate import ahead of the std group in four files, a split `tokio::io` group, and qualified paths where aliases exist
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:3-8 (related: tests/harness.rs:3, 14-16, 495-509, 528, 558-575; tests/containment.rs:3; work/tests.rs:1, 142, 159, 239, 259, 279, 306; tests/greeting.rs:21-30, 47, 84-87, 99, 126, 245; tests/declarations.rs:34, 39, 48, 71, 275; tests/malformed.rs:26, 31, 45-47; tests/failures.rs:56; start/tests.rs:78, 101, 176; tests.rs:165, 361)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read every cited line; `grep -c 'crate::tree::Root'`: greeting.rs 9, declarations.rs 5, malformed.rs 3, failures.rs 1, while tests.rs:25, harness.rs:23, and containment.rs:11 import `Root as TreeRoot`)
- Seen by: structure-prose; refutation: confirmed; history: no rationale (the `crate::message` lines were inserted at each file head by 4356e197; no rustfmt.toml exists to regroup them)
- Owner-gated: no

Four files open with `use crate::message::{PayloadCodec, PayloadDepthLimit};` ahead of the std and external group, the trace of a mechanical insertion rustfmt does not regroup; harness.rs splits `tokio::io` into two statements around a blank line. `crate::tree::Root` is spelled out eighteen times across four sibling files while three siblings import it as `TreeRoot`; `std::io::ErrorKind`, `std::convert::Infallible`, `crate::link::{LinkParts, MemoryConnector, MemoryAcceptor, Done}`, `crate::tree::mirror::streaming::message::initiates`, `crate::tree::typed::Path`, `crate::Network`, and `before::Party` are likewise spelled at use sites. Doctrine: imports over long qualified paths except where the qualification itself informs.

Evidence:

         3	use crate::message::{PayloadCodec, PayloadDepthLimit};
         4	use serde::Serialize;
         5	use serde::de::DeserializeOwned;
         6	use std::convert::Infallible;
         7	use std::sync::Arc;
         8	use std::sync::atomic::{AtomicUsize, Ordering};

Resolution: Move the `crate::message` import into each file's crate group; merge the `tokio::io` statements in harness.rs; import `Root as TreeRoot`, `io::ErrorKind`, `Infallible`, `Done`, `LinkParts`, `MemoryConnector`, `MemoryAcceptor`, `initiates`, `Path`, `Network`, and use `nth_party` in place of `before::Party` (see remote-proxy-tests-9). Acceptance: `grep -rn 'crate::tree::Root\|std::io::ErrorKind\|std::convert::Infallible' src/tree/mirror/streaming/remote/proxy` returns only `use` lines.

### remote-proxy-tests-4: Small helpers, a constant, and the n-versus-m fixture are each defined two or more times across sibling files
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:53-54 (related: tests/harness.rs:35-36, 521-534; tests/greeting.rs:84-105, 117-118, 189-194; tests/malformed.rs:135-139; tests/failures.rs:56-68, 103-113, 296-300; tests/transport.rs:10-20, 26-35; tests/declarations.rs:48-57, 71-80, 275-287; tests.rs:331-334, 347-350, 361-366, 593-596; work/tests.rs:238-249, 305-316; src/tree/mirror/streaming/tests/stats.rs:72-75)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read every cited body; `grep 'pub fn' src/tree/arb.rs` lists no pair builder; `tree/mirror/streaming/tests/stats.rs:72-75` computes the same election through `StreamingRoot<Local>::len()`)
- Seen by: structure-prose (three findings), blind-spots, api-economics; refutation: confirmed; history: no rationale (each copy arrived by copy; `left_initiates` landed at 3e5b95df with a doc arguing for one predicate and did not retarget greeting.rs)
- Owner-gated: no

`TRANSPORT_CAPACITY = 37` is defined with an identical doc in tests.rs and harness.rs (containment.rs imports the hub's copy, showing the intended shape). `greeting::order_by_election` re-derives `harness::left_initiates` including the hand-rolled `len` closure, and malformed.rs and failures.rs inline the same ordering on top of `left_initiates`; `stats.rs` computes the count through `StreamingRoot<Local>::len()`, so the closure is a reimplementation of an existing method. `transport::plan` and `failures::plan` are two undocumented `IoPlan` builders differing in parameter order. The parked `StreamReceiver` fixture is built twice in work/tests.rs. The fixture "n unit messages on `nth_party(0)` against m on `nth_party(1)`" is built by hand in `uneven_pair` (1 vs 4), `batched_uneven_pair` (1 vs FAN+1), `opening_bulk_pair` (4 vs 8), `failures::stacked_pair` (8 vs 8), inline in transport.rs (8 vs 8), four times in tests.rs (1 vs 1), and twice in greeting.rs; tests.rs:361-362 is the one site seeding parties with `before::Party::seed()`/`fork()` instead of `nth_party`. Tests that repeat setup a helper should own; every other helper in the partition carries a doc comment and the two `plan` functions do not.

Evidence:

    tests.rs:
        53	/// Bytes buffered by each per-stream pipe before backpressure applies.
        54	const TRANSPORT_CAPACITY: usize = 37;

    harness.rs:
        35	/// Bytes buffered by each per-stream pipe before backpressure applies.
        36	const TRANSPORT_CAPACITY: usize = 37;

    greeting.rs:
        93	    let len = |root: &crate::tree::Root| {
        94	        root.root
        95	            .as_ref()
        96	            .map(|node| node.len() as u64)
        97	            .unwrap_or_default()
        98	    };
        99	    if crate::tree::mirror::streaming::message::initiates(len(&a), &a.ceiling, len(&b), &b.ceiling)

Resolution: In harness.rs, single definitions: `TRANSPORT_CAPACITY` (imported by tests.rs), `order_by_election(a, b) -> (initiator, responder)` built on `left_initiates` (adopting `order_by_election`'s equal-ceiling assertion), `left_initiates` computing its count via `StreamingRoot::<Local>::from(root.clone()).len()`, one documented `plan(..)` builder, and `disjoint_pair(left: usize, right: usize) -> (TreeRoot, TreeRoot)` (or place the pair builder in `tree::arb` beside `nth_party`, since the sibling suites build the same shapes); rewrite the named fixtures as one-line wrappers keeping their docs; in work/tests.rs, a `parked_reporter(route, stream)` helper. Acceptance: `grep -rn 'fn hash_of\|fn union_hash\|fn plan(\|const TRANSPORT_CAPACITY\|fn order_by_election' src/tree/mirror/streaming/remote/proxy` returns one hit per name; `Action::Insert(Message::new(()))` appears in the partition only inside the builder and in fixtures needing a non-uniform shape.

### remote-proxy-tests-5: The two-proxy topology is hand-rolled seven times; `harness::drive` already abstracts the axis they vary on
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:56-79 (related: tests.rs:83-111, 123-154, 158-206, 239-301; tests/harness.rs:579-618; tests/containment.rs:28-54; src/testing/transport.rs:109-120)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read all seven bodies; `grep -c 'PayloadCodec::new'`: tests.rs 10, containment.rs 2, harness.rs 2, work/tests.rs 1; `IoPlan::default()` is pass-through, `usize::MAX` chunks with no delays and no fault; the `T` parameter of `reconcile_symmetric_accepts` is instantiated only at `()` at tests.rs:352, 381, 438, 585 and of `_reordered` only at 542; crate-wide `PayloadCodec::new::<..>(PayloadDepthLimit::default())` occurs 67 times)
- Seen by: structure-prose, api-economics (two findings); refutation: confirmed; history: no rationale (each driver arrived by copy; the one constraint on a consolidation's shape is 6a2f271a, which rejected an erased test-link carrier, so a single driver must stay generic over concrete link types)
- Owner-gated: no

`Handshaking::start(Local, Root::<Local>::from(_)).window(..)` twice, `memory_with_capacity`, `RemoteHandshaking::start(Local, link, PayloadCodec::new::<T>(PayloadDepthLimit::default())).window(..)` twice, and `join!(mirror(..), mirror(..))` are written out in `reconcile`, `reconcile_symmetric_accepts`, `reconcile_symmetric_accepts_reordered`, `reconcile_after_preamble`, `reconcile_with_stacked_failures`, `harness::drive`, and `containment::reconcile_results`. They differ only in link wrapping, topology (asymmetric `mirror(a, remote_b) + mirror(remote_a, b)` versus the production `mirror(a, remote_b) + mirror(b, remote_a)`), backend (`Local` versus `Failing<Local>`), payload type, and window. `containment::reconcile_results` is behaviorally `harness::reconcile(a, b, TRANSPORT_CAPACITY, IoPlan::default(), IoPlan::default())` minus the report handles. The `PayloadCodec` argument's addition touched fourteen sites here; a harness exists so that the topology is constructed once, and a generic parameter with a single instantiation is a harness more general than its use. This finding couples with remote-proxy-tests-24: if `drive` gains the production arrangement, the consolidation should carry a topology axis rather than a second driver family.

Evidence:

        56	/// Drive two local starts, each paired directly with its remote protocol start.
        57	async fn reconcile(a: TreeRoot, b: TreeRoot) -> (TreeRoot, TreeRoot) {
        58	    let a = Handshaking::start(Local, Root::<Local>::from(a)).window(WindowConfig::FLOOR);
        59	    let b = Handshaking::start(Local, Root::<Local>::from(b)).window(WindowConfig::FLOOR);
        60	
        61	    let (a_link, b_link) = memory_with_capacity(TRANSPORT_CAPACITY);
        62	    let remote_b = RemoteHandshaking::start(
        63	        Local,
        64	        a_link,
        65	        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
        66	    )
        67	    .window(WindowConfig::FLOOR);
        ...
        75	    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(remote_a, b)));

Resolution: Make `harness::drive` the single constructor, parameterized by backend pair (`Local` or `Failing<Local>`, converting the `Root<Failing<Local>>` result as remote-proxy-tests-7 needs), a two-variant topology, payload type, and window, staying generic over the concrete link types; express the five hub helpers as wrappers that only wrap links and choose topology; delete `containment::reconcile_results` in favor of the harness with the asymmetric topology; drop `T` from `reconcile_symmetric_accepts`/`_reordered` (keep it on `reconcile_after_preamble`, whose `u64` caller exists). The codec incantation then has one site in the partition; a `pub(crate)` convenience constructor on `PayloadCodec` is a crate-wide question outside this partition. Acceptance: `RemoteHandshaking::start` has one call site in the partition; `grep -c 'PayloadCodec::new' src/tree/mirror/streaming/remote/proxy/tests.rs src/tree/mirror/streaming/remote/proxy/tests/*.rs` drops to one or two; the suite passes unchanged.

### remote-proxy-tests-6: The reordering helper docs describe an effect the property proves unreachable, and its case count is undefended
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:113-122 (related: tests.rs:502-565, 434-443; src/testing/transport.rs:775-781; src/conformance/link/tests.rs:50)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both tests and the helper; the `reorder_accepts` doc in `testing/transport.rs` sanctions "zero as a tripwire where it provably cannot")
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed (deliberate design, owner-gated); history: deliberate-and-holds (cbc4a0aa states the principle: prove the adversity fires, or say it can't and pin that with a tripwire; 59827c7a moved the count onto `CASES`; the link-transport review's R13 cites the tripwire approvingly)
- Owner-gated: yes: deleting the test or restructuring the driver reopens cbc4a0aa; reducing `CASES` changes a decided parameter

The zero-inversion tripwire is a recorded decision and the test's own doc (502-521) is candid that "as exercised this property pins no more than `reconcile_symmetric_accepts`". What the decision does not cover: the `REORDER_BATCH` doc says it is "deep enough to invert most bursts" and the helper doc promises "worst-case-legal stream reordering on both ends", both describing an effect the test proves never happens in this driver, so a reader of the helpers alone is misled; the doc mixes modalities ("provably" at 504, "verified empirically" at 510-511) without saying which is the argument of record; and the 48 cases were chosen as "fewer than the default", not as the minimum that keeps the tripwire alive, although the argument is structural (any session opening two or more streams would show an inversion if the topology admitted one), so one deterministic case suffices to detect a topology change.

Evidence:

       113	/// Arrivals held and released newest-first by the reordering acceptor: deep
       114	/// enough to invert most bursts, small enough that batching never starves a
       115	/// stream.
       116	const REORDER_BATCH: usize = 3;
       117	
       118	/// [`reconcile_symmetric_accepts`] with both acceptors delivering arrivals
       119	/// in reversed batches: worst-case-legal stream reordering on both ends.

       502	/// Wide-budget divergence still matches the materialized oracle with both
       503	/// acceptors decorated for reversed-batch delivery at one-byte windows —
       504	/// with the honest caveat that the reordering provably never fires here.

Resolution: Ungated: rewrite the `REORDER_BATCH` and helper docs to state that no inversion is reachable under the joined-endpoint driver and that the counter's zero is the assertion; settle "provably" versus "verified empirically" by stating the mechanism argument once and calling the zero an observation that pins it. Gated, the owner's choice: keep the property as is; or keep the tripwire as one deterministic case over `early_first_child_dispute_pair` asserting `reordered == 0`, relying on `wide_symmetric_accepts_match_local` for the wide property and the conformance suite's `ReversingAcceptor` for the decorator; or restructure the driver so accepts can genuinely batch and flip the tripwire to `> 0`. Acceptance: the helper docs and the test doc agree that no inversion is reachable and say why; either the case count is one with the reason stated, or the current count is defended in the `CASES` doc.

### remote-proxy-tests-7: The backend-fault property discards the unfaulted endpoint's tree, so a divergent completion passes
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:296-300 (related: tests.rs:445-499, 219-224; tests/failures.rs:244-253)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the helper and the property: the only uses of `result.0`/`result.1` are error matching and `is_ok()`; the transport dual at failures.rs:244-253 compares any `Ok` counterparty to the oracle)
- Seen by: blind-spots; refutation: confirmed; history: no rationale (`left.map(|_| ())` is from cbfe1aff; 6410212a added the transport dual's stronger check and did not revisit this property)
- Owner-gated: no

`reconcile_with_stacked_failures` maps both results to `()`, so `proxy_backend_failures_are_fail_fast` checks only the faulted side's error identity plus quiescence. An unfaulted counterparty that completes `Ok` with a tree different from the oracle is indistinguishable from a correct one, and a backend failure injected after the counterparty absorbed a partial supply run is exactly the moment a wrong-but-complete tree could emerge. The doc's "terminates both endpoints" is pinned only by liveness; the mirror's contract that an `Ok` endpoint returns the reconciled root is not pinned here at all, while its transport sibling pins it.

Evidence:

       296	    let (left, right) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(remote_a, b)));
       297	    (
       298	        (left.map(|_| ()), right.map(|_| ())),
       299	        if fail_left { a_io } else { b_io },
       300	    )

       445	    /// Every reached proxy backend failure terminates both endpoints and
       446	    /// survives transport cancellation with its exact operation identity.

Resolution: Return the roots from the helper (`left.map(|(root, _)| ..)`, `right.map(|(_, root)| ..)`), adding the inverse of `failing_root` (tests.rs:219-224) to convert `Root<Failing<Local>>` back to `TreeRoot`; compute `reconcile_locally(a, b)` in the property and assert any `Ok` result equals the oracle's side, as failures.rs:247-253 does. `stacked_backend_and_transport_failures_remain_distinct` can keep discarding at its own call sites. Acceptance: `proxy_backend_failures_are_fail_fast` contains a `prop_assert_eq!` against the local oracle for whichever endpoint returns `Ok` when the injection fired, and the helper's return type carries `TreeRoot`.
Construction: Wrap the unfaulted proxy's backend to substitute one leaf's payload after N operations while the faulted side fails; the current property passes because the tree is never compared.

### remote-proxy-tests-8: Two session-driving tests run on pollster, so a stall would hang to the nextest kill instead of failing as `Stalled`
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:316-330 (related: tests.rs:345-355; src/testing.rs `run_to_quiescence`; .config/nextest.toml:25-26)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read; every other session in the partition goes through `run_to_quiescence`; nextest terminates after three 60-second periods)
- Seen by: api-economics; refutation: confirmed; history: no rationale (both were `#[tokio::test]` at cbfe1aff; 83edcd94, a work-in-progress commit, switched them to pollster while `run_to_quiescence` was already in use by siblings in the same file)
- Owner-gated: no

`equal_versions_return_both_roots` and `divergent_leaves_converge` are `#[pollster::test]` over `reconcile(..)`, a full two-proxy session on memory links. Under pollster a reintroduced stall parks the thread until nextest's 180-second termination and reports no cause; under `run_to_quiescence` it is `Err(Quiescence::Stalled)` with the stalled future named. The determinism story in `.config/nextest.toml` is applied inconsistently to the two smallest sessions, the ones a stall regression reaches first. (`start/tests.rs`'s pollster uses read from `&[u8]` and cannot stall; no change is needed there.)

Evidence:

       316	/// Equal versions close every unused logical stream without opening descent.
       317	#[pollster::test]
       318	async fn equal_versions_return_both_roots() {

       328	/// Concurrent version-addressed leaves cross every proxy layer and converge.
       329	#[pollster::test]
       330	async fn divergent_leaves_converge() {

Resolution: Rewrite both as `#[test] fn ... { let (a, b) = run_to_quiescence(reconcile(..)).expect("..."); ... }`, matching `symmetric_accept_handshakes_are_live`. Acceptance: `grep -n pollster src/tree/mirror/streaming/remote/proxy/tests.rs` is empty.

### remote-proxy-tests-9: The preamble-then-session byte-isolation claim is tested only under a name and doc about payload types
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:357-371 (related: tests.rs:156-158)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read; `reconcile_after_preamble`'s sole caller is line 368)
- Seen by: api-economics; refutation: confirmed; history: no rationale (the shape is from 83edcd94; `nth_party` existed since 1af0ab9a)
- Owner-gated: no

`reconcile_after_preamble` documents itself as "proving that neither phase consumes the other's bytes", and its only caller is `symmetric_accepts_with_distinct_payloads_are_live`, whose name and doc speak only of payloads. A reader auditing preamble/session byte isolation will not find the test; a reader of the test will not know the preamble is under test. The test also builds its parties with `before::Party::seed()`/`fork()` where every sibling uses `nth_party`. A testdoc states the invariant the test protects and must be accurate.

Evidence:

       357	/// Distinct payloads exercise supplied-leaf paths different from the unit
       358	/// payload used by the broad protocol properties.
       359	#[test]
       360	fn symmetric_accepts_with_distinct_payloads_are_live() {
       361	    let mut a_party = before::Party::seed();
       362	    let b_party = a_party.fork();
        ...
       368	    let (a, b) = run_to_quiescence(reconcile_after_preamble::<u64>(a.root, b.root))

       156	/// Drive the production proxy topology after the shared preamble on the same
       157	/// transport halves, proving that neither phase consumes the other's bytes.

Resolution: Either split into a `preamble_and_session_share_the_control_halves` test (documenting the byte-isolation claim, unit payload) and run the payload test on `reconcile_symmetric_accepts::<u64>`, or fold the preamble claim into this test's name and doc. Use `nth_party(0)`/`nth_party(1)`. Acceptance: the doc of whichever test calls `reconcile_after_preamble` states the byte-isolation claim; no `Party::seed()` in the file.

### remote-proxy-tests-10: No frame ever crosses a link on a logical stream with index two or greater, anywhere in the crate
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:388-394 (related: tests.rs:26; src/tree/arb.rs:235-237, 567, 625; src/tree/mirror/streaming/tests/fixtures.rs:475, 494; src/tree/mirror/streaming/remote/codec/signal.rs:55-77; src/tree/mirror/streaming/driver.rs:161-171; src/tree/mirror/streaming/remote/proxy/state.rs:89-91; tests/snapshots/*.snap)
- Class / severity / confidence: verification-gap / high / high
- Provenance: demonstrated (second witness pass: an observer counting frames per sent stream index over the in-memory link showed a 24-leaf content-addressed divergence opening only stream indices 0 and 1 on either side, matching the committed snapshots, while both deep fixtures opened streams 0..=16 and failed over the wire with `Decode(LeafOutsideScope ..)`, so the gap holds and the proposed closing test cannot close it as written; before the pass, verified by grep: `grep -rhoE '(Initiator|Responder) stream [0-9]+ \(height [0-9]+\)' tests/snapshots | sort | uniq -c` returns only stream 0 at height 31, `Initiator stream 1 (height 30)`, and `Responder stream 1 (height 29)`; `RemoteHandshaking` is constructed outside this partition only in streaming.rs, peer/gossip.rs, and peer/gossip/tests.rs, none of which uses a deep fixture; every fixture in this partition is content-addressed or `early_first_child_dispute_pair`)
- Seen by: blind-spots; refutation: reframed (the leaf-tier functions do run on every session; what has no wire witness is frames on streams >= 2); history: no rationale (the deadlock note records this tier's generator distribution being found too thin for the deadlock's width geometry and closed at b3b877d9 with `early_first_child_dispute_pair`; nothing extends that to depth; `git log -S'leaf_parent_dispute_pair' -- src/tree/mirror/streaming/remote` is empty)
- Owner-gated: no

Every fixture reaching the proxy tier is content-addressed: SHA3 scatters leaf paths at the root fan, so disputes resolve within the first two heights, and successive streams descend two heights each, so logical streams 2..16 in either direction never carry a frame. The proxy's per-height stages do execute on every session (`mirror_connected` steps every reply round, and `stream_at::<H>` runs for every height), but with empty request sets; the stream opens at indices >= 2, the leaf-tier decode pump on live traffic, and the `LeafParentReplies`/`TerminalLeafReplies` grammar arms validating real frames are exercised over a link by nothing, and the committed public snapshots are the mechanical statement of the gap. The deep fixtures that exist (`leaf_parent_dispute_pair`, `leaf_parent_redaction_pair`, both `pub` in `tree::arb` and returning their expected union; `full_depth_comb_pair`, `pyramid_pair`, `pub(super)` in the streaming tests) run only through two in-process participants. The doc's "arbitrary valid divergence" overstates what the generator reaches. Correct at all scales: a wrong speaker/height parity for a deep stream, a mis-wired leaf reply, or a placement-grammar bug on the terminal streams would pass every test in the crate that runs over a link.

Evidence:

       388	    /// For arbitrary valid divergence, crossing the codec and per-stream
       389	    /// transport is observationally identical to the in-process protocol.
       390	    #[test]
       391	    fn wire_reconciliation_matches_local(
       392	        (a, b) in arb_divergent_pair(),
       393	        schedule in vec(0_u8..=2, 0..128),
       394	    ) {

    src/tree/arb.rs:
       235	/// Content-addressed generators cannot produce this shape — SHA3-256 scatters
       236	/// their keys at the root fan, so a merge's divergent descent below the
       237	/// root is reachable only through chosen paths like these. Both sides

Resolution: Drive the deep fixtures through the production topology: add tests calling `reconcile_symmetric_accepts::<()>` on `leaf_parent_dispute_pair()` and `leaf_parent_redaction_pair()` (their third element is the oracle; also compare to `reconcile_locally`), widen `full_depth_comb_pair`/`pyramid_pair` to `pub(crate)` or move them beside the leaf-parent fixtures and drive them too, and add a proptest over `arb_deep_divergent_pair()` at this tier. Re-denominate the `wire_reconciliation_matches_local` doc to the shapes it samples, or add the deep generator to it. Acceptance: a test in this partition, driving `RemoteHandshaking` over a link, observes frames on a logical stream with index >= 2 in each direction (via `IoReport.connects >= 3` per side, or a hook capture) with reconciled roots equal to the oracle; the leaf-parent dispute and redaction fixtures both converge over the wire; a wire snapshot renders a stream header deeper than stream 1.
Construction: `let (a, b, union) = leaf_parent_dispute_pair(); let (l, r) = run_to_quiescence(reconcile_symmetric_accepts::<()>(a, b, TRANSPORT_CAPACITY)).expect(..); assert_eq!((l, r), (union.clone(), union));` If it passes, commit it and the gap is closed; if it fails, a wire-only defect has been found. Outcome (second witness pass): it fails, and neither branch applies; the Witness paragraph below explains.

Witness: the second witness pass ran this construction and a frame census beside it (`witness/results.md`, `## remote-proxy-tests-10`). Two tests appended to proxy/tests.rs called the unmodified `reconcile_symmetric_accepts::<()>` on `leaf_parent_dispute_pair()` and `leaf_parent_redaction_pair()` after first checking each fixture against `reconcile_locally`; a third, diagnostic test attached a crate-private observer that counts frames per sent data-stream index and ran the content-addressed baseline (24 leaves a side) and both deep fixtures at `TRANSPORT_CAPACITY` and at 64 KiB. The committed snapshots, grepped mechanically, name only `stream 0 (height 31)` (22 times), `stream 1 (height 29)` (twice), and `stream 1 (height 30)` (seven times). Decisive output:

    WITNESS remote-proxy-tests-10 [content-addressed baseline, capacity 37]: frames per sent stream index: side A {0: 19, 1: 27}; side B {0: 43, 1: 5}; highest stream index opened = 1
    WITNESS remote-proxy-tests-10 [leaf_parent_dispute_pair, capacity 37]: frames per sent stream index: side A {1: 2, 2: 2, 3: 2, 4: 2, 5: 2, 6: 2, 7: 2, 8: 2, 9: 2, 10: 2, 11: 2, 12: 2, 13: 2, 14: 2, 15: 2, 16: 4}; side B {0: 2, 1: 2, 2: 2, 3: 2, 4: 2, 5: 2, 6: 2, 7: 2, 8: 2, 9: 2, 10: 2, 11: 2, 12: 2, 13: 2, 14: 2, 15: 2}; highest stream index opened = 16
    WITNESS remote-proxy-tests-10 [leaf_parent_dispute_pair, capacity 37]: side A Err(Server(Stream(SupplyClosed { origin: Stream { speaker: Responder, stream: Stream(16) }, source: Some(Custom { kind: UnexpectedEof, error: "peer link is gone" }) }))); side B Err(Server(Decode(LeafOutsideScope { expected: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], actual: [84, 185, 194, 209, 144, 126, 178, 114, 158, 214, 57, 99, 201, 76, 40, 254, 51, 214, 176, 221, 37, 187, 136, 235, 47, 123, 221, 17, 135, 50, 207, 39] })))
            FAIL [   0.016s] (2/3) rumors tree::mirror::streaming::remote::proxy::tests::zz_witness_leaf_parent_dispute_pair_over_wire
            FAIL [   0.016s] (3/3) rumors tree::mirror::streaming::remote::proxy::tests::zz_witness_leaf_parent_redaction_pair_over_wire
         Summary [   0.069s] 3 tests run: 1 passed, 2 failed, 838 skipped

Verified by running: the content-addressed baseline opens only stream indices 0 and 1 on either side, matching the committed snapshots; both deep fixtures pass the in-process oracle and open streams 0..=16 over the link, so the deep tiers are reachable in principle; over the wire both fail, at either capacity, with the receiving side's primary error `Decode(LeafOutsideScope ..)` (the other side's `SupplyClosed` follows the failed side dropping its link). Assessed by reading in the same pass: `LeafOutsideScope` is raised at src/tree/mirror/streaming/remote/adapter/decode.rs:503-512, where the receiver derives the leaf's path with `Path::for_leaf(version)` and checks it against the reply scope, and the fixtures place leaves at `leaf_sibling_path` (src/tree/arb.rs:503-513), paths no version-addressed leaf can have. So the construction's failure is neither of the branches it names: the fixtures violate the version-addressing invariant the wire relies on, the wire correctly refuses them, and closing the gap needs a different vehicle, either a content-addressed deep divergence found by hash-prefix search (reachable only to modest depth) or a codec and stream-tier harness that feeds deep-indexed frames directly. The edits were restored afterwards.

### remote-proxy-tests-11: Test docs narrower than their bodies, one hand-maintained count, and one mux-era clause
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:388-405 (related: tests.rs:316, 343-344; tests/failures.rs:158-166, 275-277, 327; start/tests.rs:277-278, 288, 296-300; tests/greeting.rs:174-178)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read each doc against its body; production call sites of `streaming::handshake` are gossip.rs:1152 and 1210)
- Seen by: structure-prose, blind-spots; refutation: confirmed; history: no rationale (`payload_depth_limit: 256` was added by 71de90c1 with no assertion; "close every unused logical stream" dates from cbfe1aff's mux era and expired at b3b877d9, when streams became lazily opened; 6410212a relaxed the counterparty clause for the generated property only)
- Owner-gated: no

`wire_reconciliation_matches_local`'s doc claims observational identity only, but the body also asserts trace validity and one-slot channel bounds. `every_transport_fault_surface_is_reachable` asserts `outcome.right.is_err()` while its doc says nothing about the counterparty and the neighbouring property's doc explicitly permits the counterparty to complete; the mechanism (the fault fires at `after: 0`, before any exchange, so the counterparty cannot have completed) is unstated. `empty_listing_greeting_decodes` says "the decoded handshake carries the sent fields" but checks four of the five it sets, omitting `payload_depth_limit`. `symmetric_accept_handshakes_are_live` says "used by both public API endpoints", a hand count. `equal_versions_return_both_roots` says equal versions "close every unused logical stream", a clause from the multiplexed era; today no stream exists to be closed, and greeting.rs:176-177 asserts `connects == 0` and `accepts == 0`. AGENTS.md: an inaccurate testdoc is a bug in the test; doctrine: no hand-maintained counts.

Evidence:

       388	    /// For arbitrary valid divergence, crossing the codec and per-stream
       389	    /// transport is observationally identical to the in-process protocol.
        ...
       402	        trace.assert_valid();
       403	        assert_proxy_channels_are_bounded(&channels);

    start/tests.rs:
       288	        payload_depth_limit: 256,
        ...
       296	    assert_eq!(handshake.version, version);
       297	    assert_eq!(handshake.set_len, 7);
       298	    assert_eq!(handshake.max_version_bytes, 512);
       299	    assert_eq!(handshake.target_message_size, 1 << 16);
       300	    assert!(handshake.listing.is_empty());

    failures.rs:
       327	        assert!(outcome.right.is_err());

    tests.rs:
       343	/// The same client/proxy pairing used by both public API endpoints remains
       344	/// live under deterministic closed-world polling.

Resolution: Add the asserted clauses to each doc (trace validity and channel bounds; counterparty failure under an immediate fault, with the `after: 0` argument); assert `handshake.payload_depth_limit == 256`; replace "both public API endpoints" with "the pairing the session drivers use"; narrow `equal_versions_return_both_roots`'s doc to root equality with no stream opened. Acceptance: each doc lists every property its body asserts; the greeting test asserts all five sent fields.

### remote-proxy-tests-12: `context_registration_is_causal` reruns `wire_reconciliation_matches_local`'s exact session to add one assertion
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:407-427 (related: tests.rs:388-405, 609-623)
- Class / severity / confidence: performance / nit / high
- Provenance: verified (read: identical generators and `instrumented_reconcile` call; the second adds only `trace.assert_registration_causality()`)
- Seen by: api-economics; refutation: confirmed, downgraded (separate properties give separate failure attribution and seeds); history: no rationale (675e2f53 argues for the assertion, not for a separate case budget)
- Owner-gated: no

Both properties draw `(a, b) in arb_divergent_pair()` and `schedule in vec(0_u8..=2, 0..128)` and call `instrumented_reconcile`; at the default 256 cases the second is 256 redundant instrumented sessions whose only independent value is its (good) doc paragraph, which can ride the merged test. The counterweight is real but small: a separate property attributes a failure to registration causality by name and persists its own seed.

Evidence:

       417	    #[test]
       418	    fn context_registration_is_causal(
       419	        (a, b) in arb_divergent_pair(),
       420	        schedule in vec(0_u8..=2, 0..128),
       421	    ) {
       422	        let (result, _channels, trace) = instrumented_reconcile(a, b, schedule);
        ...
       426	        trace.assert_registration_causality();

Resolution: Call `trace.assert_registration_causality()` inside `wire_reconciliation_matches_local` after `trace.assert_valid()`, move the receive-side ordering paragraph into that doc, and delete the second property; or keep both and accept the cost knowingly. Acceptance: one property over `instrumented_reconcile` asserts oracle equality, channel bounds, send-side ordering, and registration causality, and its doc names all four.

### remote-proxy-tests-13: Module docs under-describe their files; `work/tests.rs` has none
- Where: src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:4-5 (related: declarations.rs:87-91, 94, 148; tests/harness.rs:1, 116-282, 284-446, 521-534; work/tests.rs:1)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the three files in full)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no rationale (declarations.rs's doc named the two words that existed at 739e4d1f; the `target_message_size` tests arrived with ed0f1775 without extending it; harness.rs's doc is from 77674c9c and predates the rewriter and the election predicate; work/tests.rs has opened with a `use` since cbfe1aff)
- Owner-gated: no

declarations.rs's module doc lists the size words as `set_len` and `max_version_bytes`, but two tests rewrite `target_message_size`, and the doc at 87-91 itself says that "complet[es] the declaration matrix beside `set_len` and `max_version_bytes`". harness.rs:1 says "for transport-adversity properties" while the file also provides frame scripting, greeting rewriting, and the election predicate used by three sibling suites. work/tests.rs is the only partition file with no `//!` line. A module doc's first sentence stands alone in a listing; a doc naming two of three inputs misleads the reader who came for the third.

Evidence:

    declarations.rs:
         4	//! The greeting's size words — `set_len`, `max_version_bytes` — are
         5	//! peer-declared inputs to the window solve and the role election. These

    harness.rs:
         1	//! Reusable two-proxy session harness for transport-adversity properties.

    work/tests.rs:
         1	use crate::message::{PayloadCodec, PayloadDepthLimit};

Resolution: declarations.rs: name `target_message_size` (the run budget's input) beside the other two. harness.rs: "Two-proxy session harness: link decorators (I/O plans, frame scripts, greeting rewrites), the shared driver, and the role-election predicate." work/tests.rs: a `//!` line naming the `Work` executor's error-selection properties. Acceptance: each doc names what its file contains; `grep -L '^//!' -r --include='*.rs' src/tree/mirror/streaming/remote/proxy` lists no test file.

### remote-proxy-tests-14: Adversary register ("lie", "lied", "deceived") in a suite whose own module doc names the regime buggy-peer
- Where: src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:8-11 (related: declarations.rs:84, 87, 97, 113, 121, 143, 146, 219, 227, 249, 257, 289, 321, 329, 373; start/tests.rs:12, 88, 188-189; 43 further sites in ten files outside the partition, including the `GreetingLie` type in src/tree/mirror/streaming/testing/faulting.rs re-exported at streaming.rs:68)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (grep of `lie|lied|lies|deceived|deceive` as whole words across src and tests; malformed.rs:192 "where they lie" is the verb of position and is excluded)
- Seen by: structure-prose; refutation: confirmed (prose-register call); history: no rationale for the survivors (408ede87 chose "mis-declared" as the crate's register for this concept and applied only its victim-to-receiver half to declarations.rs)
- Owner-gated: yes: a crate-wide vocabulary sweep that includes a test-scaffold type name is the owner's prose call

The module doc frames the sessions as "the buggy-peer regime ... (an authorized peer already holds write authority, so none of this is a security boundary)" and in the next sentence says "what a lied declaration costs"; the tests continue with "the deceived side", "undetected set_len lie" panic messages, and "the understated lie". AGENTS.md's model of record puts hostile-peer regimes off-model and makes the violation machinery a conformance-bug detector; lie and deceive presuppose intent the model excludes, so the vocabulary contradicts the paragraph it sits in, and "a lied declaration" is not idiomatic English. The recorded register already exists: 408ede87 established "mis-declared". The same vocabulary appears crate-wide, so a partition-local fix would leave the crate split.

Evidence:

         8	//! declaration the peer's actual traffic does not honor: the buggy-peer
         9	//! regime, exercised as a conformance tripwire (an authorized peer already
        10	//! holds write authority, so none of this is a security boundary). Each
        11	//! test pins what a lied declaration costs the receiving side.

Resolution: Sweep to the recorded register: "misdeclared"/"under-declared"/"a declaration its traffic does not honor"; "the side hearing the misdeclaration" for "the deceived side"; panic messages "undetected set_len misdeclaration: ...". Do it crate-wide in one prose commit, renaming `GreetingLie` in the same pass; drop the secondary moralizers "genuinely batched" (declarations.rs:70) and "genuinely pristine" (greeting.rs:181) while there. Acceptance: `grep -rn -i -w 'lie\|lied\|lies\|deceived\|deceive' src tests` returns only positional uses of the verb.

### remote-proxy-tests-15: Hash-only convergence assertions never check the reconciled ceiling
- Where: src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:33-43 (related: declarations.rs:164-165, 365-366, 393-394; tests/greeting.rs:20-23, 45-51, 63-64, 77-78, 137-143, 171-172, 203-204; tests.rs:335-340; src/tree.rs:113-117, 131-135, 274-278; src/tree/typed/node.rs:410-417; src/tree/mirror/streaming/tests.rs:157)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read `Tree::hash`, which hashes `Node::root_hash(&Option<Root>)` over the nodes alone after `From<Root> for Option<node::Root>` drops the ceiling; `Root: PartialEq` compares `self.ceiling == other.ceiling && self.root == other.root`; the hub's properties compare whole `TreeRoot` values against `reconcile_locally`)
- Seen by: blind-spots, api-economics, structure-prose (the duplicated helpers); refutation: confirmed, downgraded (the hub already pins the ceiling for arbitrary divergence at this tier); history: no rationale (`hash_of` was introduced at 0aa29ed9 when `Root: PartialEq` already compared the ceiling and `divergent_leaves_converge` beside it asserted whole-root equality; 739e4d1f copied the helpers into declarations.rs)
- Owner-gated: no

`hash_of` reduces a `tree::Root` to `Tree::hash()`, which covers the nodes only; the root ceiling rides outside the nodes and `Root: PartialEq` compares it separately. Seven fixture tests (greeting.rs: both `carried_listing_*`, `empty_carried_listing_asks_for_everything`, `converged_session_carries_listings_unused`, `mixed_empty_and_populated_converges`; declarations.rs: the three `overstated_*_still_converge*`) therefore pass if the session merges content correctly but returns a wrong ceiling. In `empty_carried_listing_asks_for_everything` the initiator's tree is empty and its version is not, so the one thing it brings to the session is the one thing not asserted. The ceiling is the deletion mechanism (redaction leaves no tombstones: a wrong ceiling mis-honors later redactions). `hash_of`/`union_hash` are also verbatim duplicates across the two files, and `union_hash` reimplements the private `streaming/tests.rs::join_oracle`. The hub's whole-root oracle already pins the ceiling for generic divergence, which is why this is low rather than medium.

Evidence:

        33	/// The observable root hash of a reconciled `tree::Root`.
        34	fn hash_of(root: &crate::tree::Root) -> [u8; MERKLE_HASH_LEN] {
        35	    Tree::<()>::from_root(root.clone()).hash()
        36	}
        37	
        38	/// The expected reconciled union, computed by the in-memory join oracle.
        39	fn union_hash(a: &crate::tree::Root, b: &crate::tree::Root) -> [u8; MERKLE_HASH_LEN] {
        40	    let mut union = Tree::<()>::from_root(a.clone());
        41	    union.join(Tree::from_root(b.clone()));
        42	    union.hash()
        43	}

    src/tree.rs:
       131	impl PartialEq for Root {
       132	    fn eq(&self, other: &Self) -> bool {
       133	        self.ceiling == other.ceiling && self.root == other.root
       134	    }
       135	}

Resolution: Add one `join_oracle(a: &TreeRoot, b: &TreeRoot) -> TreeRoot` to harness.rs (built on `Tree::join`, as tests.rs:335-340 does) and assert `assert_eq!(left, expected)` on whole roots in the seven tests; delete both `hash_of`/`union_hash` pairs. Acceptance: no convergence assertion in greeting.rs or declarations.rs goes through `Tree::hash()` alone; a deliberately wrong ceiling on one reconciled side fails the test.
Construction: Patch the proxy's equal or diverged completion to return the local pre-session ceiling instead of the merged one; `empty_carried_listing_asks_for_everything` still passes because both hashes equal `populated.hash()`.

### remote-proxy-tests-16: The endpoint-to-`MirrorError`-side projection is written seven times, with two polarities
- Where: src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:109-125 (related: declarations.rs:100-104, 153-157, 188-198, 236-240, 245-261, 308-312, 317-333, 382-386; tests/failures.rs:128-149, 353-356, 380-383; tests/malformed.rs:42-59; tests.rs:469-483; tests/harness.rs:610-616)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read all sites; `endpoint_error`'s flag names the reporting side while `receiving_error`'s names the corrupt side)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale (both helpers were born in 77674c9c; the four inline copies were added by copy; the polarity split has no recorded reason)
- Owner-gated: no

Selecting the proxy error from a `(left, right)` result pair (left's proxy is `MirrorError::Server`, right's is `MirrorError::Client`, a consequence of `harness::drive`'s arrangement) appears inline in four declarations tests and in tests.rs, and as two differently named helpers, `failures::endpoint_error` and `malformed::receiving_error`, whose flags mean opposite things. The five `((left, right), hears) = if receiver_left {..}` arrangements in declarations.rs are the same shape again. The Server/Client-by-position rule is a fact about the harness topology and belongs beside `drive`, stated once; every copy is a place a topology change (remote-proxy-tests-24) breaks independently.

Evidence:

       109	        let receiver_error = if receiver_left {
       110	            match &left {
       111	                Err(MirrorError::Server(error)) => error,
       112	                other => panic!(
       113	                    "undetected target_message_size lie: the left proxy did not \
       114	                     report the violation: {other:?}"
       115	                ),
       116	            }
       117	        } else {
       118	            match &right {
       119	                Err(MirrorError::Client(error)) => error,
       120	                other => panic!(
       121	                    "undetected target_message_size lie: the right proxy did not \
       122	                     report the violation: {other:?}"
       123	                ),
       124	            }
       125	        };

    malformed.rs:
        42	/// Borrow the remote error detected opposite the corrupt writer.
        43	fn receiving_error<'a>(
        44	    corrupt_left: bool,

Resolution: Add to harness.rs a `proxy_error(side: IoSide, left: &Result<_, LeftError>, right: &Result<_, RightError>) -> &RemoteError<Infallible>` (panicking with the position and a caller-supplied context) plus a `Result<_, TestCaseError>` variant for proptest bodies, and an `arrange(receiver_left, receiver, sender, rewrite)` helper for the declaration tests; replace the seven sites, passing the reporting side everywhere so the flag means one thing. Acceptance: no `Err(MirrorError::Server(error)) => error` or `Err(MirrorError::Client(error)) => error` arm remains outside harness.rs.

### remote-proxy-tests-17: Unnamed capacities and counts: `17` four times, `64 * 1024`, `31`
- Where: src/tree/mirror/streaming/remote/proxy/tests/failures.rs:194-197 (related: failures.rs:217, 258, 309; tests.rs:164; work/tests.rs:89; src/link.rs:169; src/tree/mirror/streaming/remote/codec/signal.rs:32)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the sites; `STREAM_COUNT = 17` at link.rs:169 and `Stream::COUNT = 17` at signal.rs:32; per the history pass's blame, the `17` is from 77674c9c, one day before link.rs and `STREAM_COUNT` existed, so the coincidence is accidental)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no rationale
- Owner-gated: no

failures.rs passes the literal `17` as transport capacity four times while the partition's named capacity is `TRANSPORT_CAPACITY = 37`; because 17 also equals `STREAM_COUNT` and `Stream::COUNT`, a reader cannot tell whether the coincidence is the point (history says it is not). tests.rs:164 uses `memory_with_capacity(64 * 1024)` and work/tests.rs:89 parks `0..31` futures with no name for 31. Named constants over magic numbers: the name is where the reason lives.

Evidence:

       194	        let clean = run_to_quiescence(harness::reconcile(
       195	            left.clone(),
       196	            right.clone(),
       197	            17,

    tests.rs:
       164	    let (mut a_link, mut b_link) = memory_with_capacity(64 * 1024);

    work/tests.rs:
        89	    for _ in 0..31 {

Resolution: Use `TRANSPORT_CAPACITY` in failures.rs (17 carries no intent), or name the value with a one-line doc; name `64 * 1024` for what it buys the preamble test (an unbuffered run) and `31` (`PARKED_PUMPS`, with the reason, or a small number). Acceptance: no bare integer capacity or spawn count remains in the three files.

### remote-proxy-tests-18: The transport-failure property's recovery run reconciles immutable inputs over a fresh link and pins nothing new
- Where: src/tree/mirror/streaming/remote/proxy/tests/failures.rs:255-266 (related: failures.rs:153, 158-166, 191-205; tests/harness.rs:449-468)
- Class / severity / confidence: performance / low / high
- Provenance: verified (read: `harness::reconcile` calls `memory_with_capacity` afresh on every call at harness.rs:456; `left`/`right` are owned roots whose earlier uses took clones; the doc at 158-166 names no recovery claim)
- Seen by: api-economics; refutation: confirmed, downgraded (32 cases, so 32 redundant sessions); history: no rationale (the block was born with the property in 77674c9c, already pinning nothing beyond the clean run; 6410212a's rework left it untouched)
- Owner-gated: no

After a fault fires, the property runs a third full session with both plans default. Nothing from the faulted session can reach it: it is a clean reconcile of the same inputs, already asserted by the `clean` run and by `successful_io_adversity_matches_materialized`. The claim a recovery run would pin (a failed session leaves the replica and link usable) lives at the `Peer`/`Link` tier, where poison and epoch state persist; at this tier the harness makes it vacuously true. Principle 6: the worst passing artifact here is any implementation that passes the clean run.

Evidence:

       255	            let recovered = run_to_quiescence(harness::reconcile(
       256	                left,
       257	                right,
       258	                17,
       259	                IoPlan::default(),
       260	                IoPlan::default(),
       261	            ))
       262	            .map_err(|stopped| TestCaseError::fail(format!(
       263	                "clean recovery became quiescent: {stopped:?}",
       264	            )))?;
       265	            prop_assert_eq!(recovered.left.as_ref().ok(), Some(&expected.0));
       266	            prop_assert_eq!(recovered.right.as_ref().ok(), Some(&expected.1));

    harness.rs:
       456	    let (left_link, right_link) = memory_with_capacity(capacity.max(1));

Resolution: Delete the `recovered` block. If the intent was to reuse the same link after the fault, that is a different test: hand the faulted link with its `SessionState` to a second session and assert the poison/epoch behavior the link contract promises, beside the link-tier tests. Acceptance: the property runs at most three sessions per case (oracle, clean, faulted) and its doc names every claim it asserts.

### remote-proxy-tests-19: Em-dashes in `//` comments at three sites, instances of a crate-wide pattern
- Where: src/tree/mirror/streaming/remote/proxy/tests/failures.rs:290-292 (related: tests/greeting.rs:238; work/tests.rs:303; 169 `//` comment lines crate-wide)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn '—' src tests benches examples --include='*.rs' | grep -E ':[0-9]+:\s*//[^/!]' | wc -l` = 169; the partition's three sites listed)
- Seen by: structure-prose; refutation: reframed (crate-wide convention, not a partition anomaly); history: no rationale (the link-transport review's R76 applied the house rule to an assert message in this very file, and it is fixed; two of the three sites postdate that ruling)
- Owner-gated: no

The owner's dash-by-register rule (spaced double hyphens in chat and code comments, true em-dashes only in rendered prose) is violated in three `//` comments here and 169 crate-wide. Fixing the three alone would create inconsistency; the useful action is one mechanical sweep, which is why this is filed as a nit pointing at the crate-wide count rather than as a partition defect.

Evidence:

       290	        // The faulted (left) side must be the elected responder: the
       291	        // Accept surface's mechanism — a destroyed incoming stream
       292	        // surfacing from the receiver that provably needed it — requires

Resolution: One crate-wide sweep of `//` comments (not `///` or `//!`) replacing em-dashes with colons, semicolons, or `--`, ideally with a `tools/` lint so the rule holds. Acceptance: the grep above returns zero.

### remote-proxy-tests-20: A `Tree::join` invariant is filed in the proxy greeting suite under a name that states the failure
- Where: src/tree/mirror/streaming/remote/proxy/tests/greeting.rs:206-234 (related: greeting.rs:1-8, 250, 254-256, 264, 271; src/tree/traverse/join/tests.rs:24-51; src/tree/traverse/join.rs:140; tests/session_overlap.rs:78)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read; `join/tests.rs` holds only idempotence, commutativity, and associativity; `tests/session_overlap.rs:78` is the public end-to-end twin `overlapped_install_never_loses_innocent_messages`; the inertness of the wire calls follows from the doc's own statement at 254-256 and from equal-version sessions returning the local root, but I did not run the wire-free form)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed, downgraded (end-to-end coverage exists in tests/session_overlap.rs); history: no rationale for the tier (f5426c9d landed the test with the join fix; 4bbd6c5b deliberately preserved the body and assertions while reframing the doc, and kept the name and the `WITNESS` label, which appears nowhere else in the codebase)
- Owner-gated: no

The test asserts that joining a clone-derived fan (`ours.clone()` + `remove(r_h)`) against its original is an identity on the tree, and its own doc concludes "`Tree::join` merge-walks the two radix fans in lockstep ... this witness holds the join to that". The two `wire_reconcile` calls reconcile equal versions (returning the fork-time root handle, which `t0.root.clone()` also is) and `t0` against a redacting twin (equal to `join` by the design of record); the doc itself says "the sharing that matters is created by our install below, not here". So the proxy contributes nothing the assertion sees, the file's module doc scopes it to "the greeting-carried opening listing", a maintainer changing `join` would not look here, the name states the bug rather than the invariant, the `WITNESS (...)` header is a roster-style prefix ahead of the claim, line 234's `use crate::tree::Action;` shadows the import at line 13, and there is no blank line between 205 and 206.

Evidence:

       205	}
       206	/// WITNESS (the gossip install must merge-walk clone-derived fans):
       207	///
       208	/// Two honest, overlapping sessions at one peer must not silently delete a
       209	/// message nobody redacted.
        ...
       232	#[test]
       233	fn overlapping_sessions_lose_innocent_leaf_after_honored_redaction() {
       234	    use crate::tree::Action;

       254	        // S1's counterparty: converged at T0, then redacted the leaf at
       255	        // `k` (a local act rebuilds its own fans afresh; the sharing that
       256	        // matters is created by our install below, not here).

Resolution: Move it to `src/tree/traverse/join/tests.rs` as a unit test over `Tree::join` with the wire removed: `s2_reconciled` becomes `t0.root.clone()` and `s1_reconciled` becomes `Tree::from_root(t0.root.clone()).join(twin)`; keep the sweep over the redacted leaf and the mechanism paragraph (the desync-onto-neighboring-radixes explanation is the valuable part); rename to the invariant (`join_of_own_causal_past_is_identity_on_clone_derived_fans`); drop the `WITNESS` header so the first sentence is the claim; delete the inner `use`; consider stating it as a proptest over fan width and redacted leaf. Acceptance: greeting.rs contains only greeting-listing tests; join/tests.rs holds the clone-derived-fan identity test with no proxy imports; reintroducing a shortcut walk over shared runs (the shape f5426c9d removed) still fails it.
Construction: Copy the body without `wire_reconcile` as above; the assertion at 282-286 must still hold, and the reverted join must still fail it.

### remote-proxy-tests-21: Signal state codes are hand-copied into the harness and the malformed suite because `Signal` is not re-exported
- Where: src/tree/mirror/streaming/remote/proxy/tests/harness.rs:38-42 (related: harness.rs:185-190; tests/malformed.rs:95, 196; src/tree/mirror/streaming/remote/codec/signal.rs:235-245, 249, 261; src/tree/mirror/streaming/remote/codec.rs:73, 101-103; src/tree/mirror/streaming/remote/codec/signal/tests.rs:138)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read `Signal::STATES`: Match Continue/End = 0/1, QueryEmpty = 2/3, Query = 4/5, Supply = 6/7, End(Reply) = 8, End(Stream) = 9; `Signal::state` and `from_state` are `pub fn`; `mod signal;` is private at codec.rs:73 and the re-export list at 101-103 omits `Signal`; `state_roster_snapshot` pins the roster)
- Seen by: structure-prose, blind-spots, api-economics; refutation: reframed (the codes are wire-format constants pinned by the roster snapshot, so a renumbering fails loudly crate-wide; the `State(0)`/`State(9)` tests would also fail loudly, since their outcomes depend on the mutation's meaning; only `FrameSelector::Query`/`EndingReaction` could select different frames without their own failure); history: no rationale (3327a92b rewrote this layer to the two-item opener, kept the literals, and did not add `Signal` to the re-export list)
- Owner-gated: no

`QUERY_STATES = 4..=5`, `REACTION_STATE_COUNT = 8`, the `state % 2 == 1` ending-reaction rule, `MATCH_CONTINUE_STATE = 0`, and `STREAM_END_STATE = 9` restate `Signal::STATES` by hand. They are correct today and a renumbering is an owner-ruled format change the snapshot would catch, so the cost is legibility and single-source: a frame selector that names `Signal::Query(Flow::Continue)` reads as what it is, and the `Query`/`EndingReaction` selectors would otherwise retarget silently under a roster change (a `script.fired()` witness would still hold). Doctrine: a number that matters lives in one mechanically-enforced place that prose may cite by name. The resolution as the first two lenses wrote it does not compile: `Signal` is unreachable from this partition until codec.rs re-exports it.

Evidence:

        38	/// Dense states occupied by the two nonempty-query flow variants.
        39	const QUERY_STATES: RangeInclusive<u8> = 4..=5;
        40	
        41	/// Dense states below this boundary carry reactions rather than bare ends.
        42	const REACTION_STATE_COUNT: u8 = 8;

       185	        let selected = match script.selector {
       186	            FrameSelector::First => true,
       187	            FrameSelector::State(expected) => state == expected,
       188	            FrameSelector::Query => QUERY_STATES.contains(&state),
       189	            FrameSelector::EndingReaction => state < REACTION_STATE_COUNT && state % 2 == 1,
       190	        };

    codec.rs:
       101	pub use signal::{
       102	    DecodeSignalError, End, Flow, InvalidSignalPlacement, Speaker, Stream, StreamClass,
       103	};

Resolution: Add `#[cfg(test)] pub use signal::Signal;` to codec.rs; in the harness select by `Signal::from_state(state)` (`Query` matches `Ok(Signal::Query(_))`, `EndingReaction` matches `Ok(Signal::Match(Flow::End) | Signal::QueryEmpty(Flow::End) | Signal::Query(Flow::End) | Signal::Supply(Flow::End))`); in malformed.rs write `Signal::Match(Flow::Continue).state()` and `Signal::End(End::Stream).state()`; delete the four local constants. Acceptance: no numeric state literal remains in `proxy/tests/**`; the malformed suite passes with every `script.fired()` assertion holding.

### remote-proxy-tests-22: The greeting rewriter carries no fired witness, so the three still-converge tests pass if the rewrite never applies
- Where: src/tree/mirror/streaming/remote/proxy/tests/harness.rs:284-297 (related: harness.rs:394-446; tests/declarations.rs:147-167, 350-368, 375-396)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (read: `GreetingRewrite`/`RewriteRead` expose nothing like `Script::fired()`; the three overstated tests assert only convergence, which an unrewritten session also yields)
- Seen by: api-economics; refutation: confirmed (narrow gap: the understated tests share the rewriter and fail loudly without it); history: no rationale (739e4d1f added the rewriter into a harness whose `Script` already had `fired()`)
- Owner-gated: no

`overstated_target_message_size_still_converges`, `overstated_version_bytes_still_converge`, and `overstated_set_len_from_the_bulk_side_still_converges` assert convergence; the cheapest passing artifact for them is "rewrite nothing". The understated tests exercise the same rewriter and would fail loudly if it did nothing, so the residual gap is a `u64::MAX`-specific failure to apply; still, Principle 6 asks every check to name what fails it, and the harness's other decorator already models the answer.

Evidence:

       284	/// One greeting size declaration replaced in the traffic a side receives.
       285	///
       286	/// Rewriting the *received* greeting simulates a buggy counterparty whose
        ...
       292	#[derive(Clone, Copy)]
       293	pub struct GreetingRewrite {
       294	    field: GreetingField,
       295	    /// The declaration the receiving side decodes instead of the honest one.
       296	    value: u64,
       297	}

Resolution: Give `RewriteRead` a shared `Arc<AtomicBool>` set when it enters `RewriteState::Serving` with a rewritten item, return it from `reconcile_rewritten_greetings`, and assert it in the three convergence tests (and, for symmetry, the failing ones). Acceptance: making `GreetingRewrite::apply` return its input unchanged fails all seven declaration tests.

### remote-proxy-tests-23: `capacity.max(1)` in `harness::reconcile` silences an assert for an input no caller passes
- Where: src/tree/mirror/streaming/remote/proxy/tests/harness.rs:456-456 (related: src/link.rs:556-560; tests/transport.rs:42, 78; tests/failures.rs:197, 217, 258, 309; tests/greeting.rs:34, 164)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (read every caller: 1, `1usize..=64`, 17, `usize::MAX`; `memory_with_capacity` asserts `capacity > 0` with a documented `# Panics`)
- Seen by: api-economics, blind-spots; refutation: confirmed; history: deliberate-but-expired (the clamp guarded `tokio::io::duplex(capacity.max(1))` at 77674c9c, where zero would hang; b3b877d9 replaced the duplex with `memory_with_capacity`, whose assert makes the clamp dead and rule-inverting)
- Owner-gated: no

The clamp never fires, and if a future caller passed zero it would be quietly reinterpreted as one where the link would have failed loudly. A guard must name a constructible failure it catches; this one converts a programmer error into silence.

Evidence:

       456	    let (left_link, right_link) = memory_with_capacity(capacity.max(1));

    link.rs:
       559	pub fn memory_with_capacity(capacity: usize) -> (MemoryLink, MemoryLink) {
       560	    assert!(capacity > 0, "a link stream must buffer at least one byte");

Resolution: Pass `capacity` through unchanged. Acceptance: `memory_with_capacity(capacity)` at that line; suite green.

### remote-proxy-tests-24: The harness arrangement puts the right proxy in the Client position, which production never does
- Where: src/tree/mirror/streaming/remote/proxy/tests/harness.rs:610-613 (related: tests.rs:75, 296; tests/containment.rs:49, 68-69, 86-87, 100; tests/harness.rs:44-48; src/tree/mirror/streaming/remote/proxy/start.rs:112-114, 130-155, 157-197, 199-219; src/tree/mirror/streaming.rs:143-153, 156-172; src/peer/gossip.rs:1148-1152, 1207-1210; src/conformance/backend.rs:710-712)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`mirror(client, server)` at streaming.rs:143-146; `handshake` calls `client.connect()` then `server.accept()` at 165-168; production calls `streaming::handshake(local, proxy)` at gossip.rs:1152 and 1210, so the proxy is always the Server; `drive` runs `mirror(remote_left, right)`, as do tests.rs:75, 296 and containment.rs:49; the remote `Connect`/`CompleteConnect` impls at start.rs:130-197 have no non-test caller; the remote `Accept` runs `try_join(send, receive)` at 217-219)
- Seen by: blind-spots; refutation: confirmed; history: deliberate-and-holds for the impls' existence (cbfe1aff: "Expose symmetric materialized and remote Handshaking entry points"; streaming.rs:29-33 documents that the drivers run any two implementors; containment.rs uses the asymmetric arrangement on purpose to pin position-independence of the materialized participant's violation report, and says so at the check site) but not for the coverage distribution
- Owner-gated: yes: dissolving `Connect`/`CompleteConnect`/`Connecting` on the remote `Handshaking` reverses cbfe1aff's design; adding the production arrangement to the harness is not gated

Every transport-adversity, malformed-frame, declaration, and greeting session runs its right endpoint as `mirror(remote_left, right)`: the proxy is the protocol Client, running `Connect` (receive the greeting first) and `CompleteConnect` (then send), a sequence production never executes, while the production-shaped concurrent `try_join(send, receive)` exchange under chunked, delayed, or rewritten control I/O runs on only one endpoint per session. The hub already names the production shape (`reconcile_symmetric_accepts`, "Drive the production topology") but only its own six tests use it. The `RightError`/`MirrorError::Client(proxy error)` arms in `receiving_error`, `endpoint_error`, and the declaration tests are test-only shapes. The asymmetric shape is a legal configuration of the generic protocol, so the tests are not wrong; the gap is that no adversity coverage runs production's wiring on both ends, and the remote `Connect` impl's only defender is containment.rs's server-position check.

Evidence:

       610	    let (left, right) = join!(
       611	        Box::pin(mirror(left, remote_right)),
       612	        Box::pin(mirror(remote_left, right)),
       613	    );

    src/tree/mirror/streaming.rs:
       165	    let (our_handshake, client) = client.connect().await.map_err(Error::Client)?;
        ...
       168	    let (peer, server) = server.accept(our_handshake).await.map_err(Error::Server)?;

    src/peer/gossip.rs:
      1152	            let handshaken = streaming::handshake(local, proxy)

Resolution: Ungated: give `drive` a topology axis (coupling with remote-proxy-tests-5) and make the production arrangement (`mirror(right, remote_left)`, mapping `right.map(|(root, _control)| root.into())`) the default for the transport, malformed, declaration, and greeting suites; keep the asymmetric arrangement for containment.rs, whose reason is documented at the check site; collapse the Server/Client projection accordingly (remote-proxy-tests-16). Gated: decide whether `impl Connect for Handshaking`, `impl CompleteConnect`, and `Connecting` in start.rs stay for containment's server-position wire check (an impl with no production caller, kept for one test) or go, with the in-process twin `uncontained_supply_is_rejected_by_streaming` carrying position-independence. Acceptance: no adversity, script, or rewrite test constructs `mirror(<RemoteHandshaking>, <materialized>)`; `grep -rn 'MirrorError::Client(' src/tree/mirror/streaming/remote/proxy` matches only materialized-side errors and containment.rs; the owner has recorded the ruling on the remote `Connect` impl.

### remote-proxy-tests-25: `bytes_past_the_stream_end_are_never_read` cannot distinguish "never read" from "read and tolerated"
- Where: src/tree/mirror/streaming/remote/proxy/tests/malformed.rs:186-196 (related: malformed.rs:212-216; tests/harness.rs:278-281; src/tree/mirror/streaming/remote/streams.rs:478-492; src/link.rs:171-199)
- Class / severity / confidence: test-quality / medium / medium
- Provenance: assessed (read: after `script.fired()` the only assertions are both sides `Ok`; the receiver breaks at `End(Stream)` and hands the half back at streams.rs:478-492; `ScriptedConnector::connect` discards the inner `Done`; the refutation pass found no test at any tier asserting the returned half's remaining bytes)
- Seen by: blind-spots; refutation: confirmed; history: no rationale (4be4b830, a work-in-progress snapshot, rewrote a test asserting the typed `AfterEnd` rejection into this liveness-only form without saying why)
- Owner-gated: no

The doc's first clause, "a duplicated stream-end frame is never read", has no assertion behind it: a receiver that read past the end control and ignored a second `End(Stream)` would also complete `Ok`. The invariant matters, as the doc says: on a reusing link those bytes are the next stream's header, so "read and tolerated" is a real bug the test would bless, and the link contract's completion clause (`Done` is invoked "exactly at the protocol's end of the data, handing the half back") has no test at any tier that inspects what remains in the handed-back half. The implementation is right today by reading; the test cannot tell.

Evidence:

       186	/// Bytes past a stream's end control belong to the transport, not the
       187	/// session: a duplicated stream-end frame is never read, and the session
       188	/// completes as if it were absent.
        ...
       195	fn bytes_past_the_stream_end_are_never_read() {
       196	    const STREAM_END_STATE: u8 = 9;

       212	        assert!(left_result.is_ok(), "left session failed: {left_result:?}");
       213	        assert!(
       214	            right_result.is_ok(),
       215	            "right session failed: {right_result:?}"
       216	        );

    harness.rs:
       278	    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
       279	        let (tx, _) = self.inner.connect().await?;
       280	        Ok((ScriptedWrite::new(tx, self.script.clone()), Done::discard()))
       281	    }

Resolution: Give the receiving side's acceptor a `Done` that captures the returned receive half (the contract hands it back resting at the frame boundary); after the session, read from the captured half and assert exactly the duplicated frame's bytes remain. Alternatively compose `wrap_link` under the scripted connector and assert the receiving side's `read_bytes` equals the sender's `write_bytes` minus the duplicate's length. Acceptance: the test asserts, by byte count or by reading the returned half, that the duplicated frame's bytes were not consumed by the session.
Construction: Temporarily make the receiver in streams.rs continue past `End(Stream)` and break on the second one; the current test still passes.

### remote-proxy-tests-26: The `Accept` arm of `Work::execute` is never resolved through `execute`
- Where: src/tree/mirror/streaming/remote/proxy/work/tests.rs:28-38 (related: work/tests.rs:41-75; src/tree/mirror/streaming/remote/proxy/work.rs:215-233; src/tree/mirror/streaming/remote/streams.rs:796-825; src/tree/mirror/streaming/remote/streams/tests.rs)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rln 'AcceptError\|Error::Accept('` over test files hits only streams/tests.rs, at the driver; work.rs:221-225 skips the post-select accept poll for `Ok(_) | Err(Error::Accept(_))`; the five tests in work/tests.rs resolve only the protocol and stream-error arms)
- Seen by: blind-spots; refutation: confirmed; history: no rationale (the arm and its comment date from 54420d7f, whose witnesses concern the deposit-versus-consequence race; no follow-up added an accept-arm witness)
- Owner-gated: no

`execute` carries a `match` arm whose stated purpose is to avoid re-polling a completed accept driver, and no test can provoke that path: a regression that polled the completed driver (an `async fn` resumed after completion panics) or let the accept arm's violation be outranked by a deposited supply failure would be caught by nothing at the session terminal. `AcceptError`'s variants (`Epoch`, `UnknownStream`, `Label`, `Duplicate`, `Unexpected`) are pinned only at the driver. A guard is justified by a concrete, constructible failure the committed tests cannot catch; the `ParkedSession` fixture already holds the live `peer: MemoryLink` needed to construct one.

Evidence:

        28	/// the way the session wires it, with everything the tests must keep alive.
        29	struct ParkedSession {
        30	    work: Work<Failing<Local>, DuplexStream, DuplexStream, MemoryAcceptor>,
        31	    /// The claim table the protocol owns in production. Keeping it alive
        32	    /// ensures no stream-layer closure can accidentally win the error race.
        33	    claims: Claims<DuplexStream>,
        34	    /// A publishing half of the error route, for tests that report to it.
        35	    route: ErrorRoute,
        36	    /// The peer link; dropping it would close the stream supply.
        37	    peer: MemoryLink,
        38	}

    work.rs:
       221	            match &outcome {
       222	                // A violation resolved the accept arm: the driver is
       223	                // complete and must not be polled again, and a violating
       224	                // driver never deposited (it returns instead of parking).
       225	                Ok(_) | Err(Error::Accept(_)) => {}

Resolution: Add a test to work/tests.rs: from `peer.into_parts()`, `connector.connect()` a stream and write a label pair whose epoch item disagrees with `session.epoch()` (or whose stream item is `>= Stream::COUNT`), then `run_to_quiescence(work.execute(future::pending()))` and assert `Err(Error::Accept(AcceptError::Epoch { .. }))` (or `UnknownStream`) with the exact values and no panic. Optionally a full-stack dual via a label mutation in `ScriptedWrite` (it already parses past the two label items) reaching `MirrorError::Server(RemoteError::Accept(_))` in malformed.rs. Acceptance: a test in work/tests.rs whose asserted outcome is `Error::Accept(_)` from `execute`.
Construction: In `parked_session()`'s peer link: connect, write two canonical unsigned-int heads with the wrong epoch, flush, then execute; expect `Error::Accept(AcceptError::Epoch { .. })`.

### remote-proxy-tests-27: `tools/testdoc` does not recognize `#[pollster::test]`, so the gate's doc requirement is unenforced for 12 tests here and 29 crate-wide
- Where: tools/testdoc:20-23 (related: tools/testdoc:81-107; justfile:183-186; AGENTS.md "Writing tests"; start/tests.rs:73; tests.rs:317, 329)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (I ran `python3 tools/testdoc` on a scratch probe holding an undocumented `#[pollster::test]`, an undocumented `#[test]`, and an undocumented `#[tokio::test]`: only the plain and tokio cases were reported, exit 1; `--self-test` passes and has no pollster case; `grep -rc '#\[pollster::test\]' src tests`: proxy/tests.rs 2, start/tests.rs 10, codec/tests.rs 1, tests/handshake.rs 7, tests/gossip_when.rs 2, tests/changes.rs 7)
- Seen by: structure-prose; refutation: confirmed by execution; history: deliberate-but-expired (the roster was complete when written at 358c6b1a on 2026-07-15; the first `#[pollster::test]` entered at 83edcd94 the next day and the roster was never revisited)
- Owner-gated: no

The lexical checker's attribute regex names `test`, `tokio::test`, `async_std::test`, `rstest`, and `test_case`. All 29 pollster tests are documented today, but an undocumented one passes `just testdoc`, and AGENTS.md's statement that "The gate's `testdoc` checks that the comment exists" is false for a whole attribute family in use. Principle 6: the cheapest passing artifact must be the intended one.

Evidence:

        20	TEST_ATTRIBUTE = re.compile(
        21	    r"^\s*#\[\s*(?:test|tokio::test|async_std::test|rstest|test_case)"
        22	    r"\s*(?:\([^]]*\))?\s*\]\s*$"
        23	)

Resolution: Match any path ending in `test` (for example `(?:[A-Za-z_][A-Za-z0-9_]*::)*test|rstest|test_case`) and add a `#[pollster::test]\nasync fn ...` case to `self_test()` so the form is pinned. Acceptance: a probe file containing an undocumented `#[pollster::test]` is reported by `./tools/testdoc <probe>`; `./tools/testdoc --self-test` passes with the new case; `just testdoc` remains clean on the tree.

## Positives

- Liveness is judged everywhere but two tests by `run_to_quiescence`, and every call site either `.expect`s the poller's result or maps it to `TestCaseError`, so a stall is a named `Quiescence::Stalled` failure rather than a hang.
- Every adversity carries a liveness witness: `Script::fired()` is asserted in all five malformed-frame tests; `transport_failures_are_exact_and_fail_fast` (failures.rs:168-272) predicts fault reachability from the clean run's own counts (`after < completed(clean_report, fault)`) and asserts `report.injected == should_inject.then_some(expected_fault)`, so the fault provably fires iff predicted; `instrumented_channels_cover_every_proxy_edge` asserts each `QueueKind::PROXY` was exercised. The api-economics lens checked the reachability predicate against `testing/transport.rs`'s fire-in-place-of-the-next-success rule for all five operations and found it agrees.
- The differential design matches the design of record: wire sessions are held to the in-process protocol (`reconcile_locally`) and, in the greeting suite, to `Tree::join`; `wire_reconciliation_matches_local` layers trace validity and one-slot channel bounds on top of result equality; the unfaulted counterparty in the transport property is bounded, not condemned, and any completion must equal the oracle exactly.
- Role-sensitive tests derive the initiator from the production `message::initiates` through `harness::left_initiates`, and harness.rs:512-520 says why byte-order guesses would rot, so fixtures survive changes to version encoding and content addressing.
- `ScriptedWrite` locates the frame's state item by parsing with the wire's own head grammar (`cbor::read_head`, harness.rs:162-184) rather than byte offsets, and every scripted test asserts `script.fired()`, so a selector that stops matching fails loudly instead of degrading to pass-through.
- `understated_target_message_size_fails_the_session` constructs its multi-record run by pigeonhole (`BULK_MESSAGES = FAN + 1`, declarations.rs:59-62), matching the budget rule that admits a lone record at any budget; the test cannot pass for the wrong reason. `understated_set_len_fails_the_session`'s doc (222-228) explains why only election-preserving rewrites model a real under-declaring peer.
- `start/tests.rs` states at the check site (lines 9-14) why it tests the internal `receive` entry, exactly the sanctioned form for an internal-entry check, and every rejection test names the exact typed error and its cause.
- `work/tests.rs` reaches the terminal's three attribution arms (deposit outranks consequence, queued stream-granularity report outranks deposit, backend error survives) deterministically by constructing the dead supply before the first poll, with docs that state the attribution contract each one pins.
- The reordered-accepts property is candid that its adversity never fires and turns that into a tripwire; whatever its disposition, the doc does not oversell the coverage.

## Open questions for Finch

- The literal `17` transport capacity in failures.rs: history says it predates `STREAM_COUNT` by a day, so the coincidence is accidental. Recommendation: replace with `TRANSPORT_CAPACITY` rather than coining a second named capacity.
- `wide_symmetric_accepts_reordered_match_local`: keep the zero-inversion tripwire as is, reduce it to one deterministic case over `early_first_child_dispute_pair`, or restructure the driver so accepts can genuinely batch? Recommendation: one deterministic case plus the helper-doc fix; the structural argument makes one case as strong a tripwire as 48, and the decorator's genuine coverage stays with the conformance `ReversingAcceptor` tests.
- Where should a shared `disjoint_pair(n, m)` fixture live: `proxy/tests/harness.rs` (this partition only) or `tree::arb` beside `nth_party` (the sibling suites build the same shapes)? Recommendation: `tree::arb`.
- Once the harness runs production's arrangement by default, `impl Connect for Handshaking` on the remote proxy has one caller: containment.rs's server-position wire check. Keep an impl with no production caller for one test, or dissolve it and let the in-process twin carry position-independence? Recommendation: dissolve, unless the wire path is judged to add something the in-process twin cannot show.
- Should codec.rs re-export `Signal` under `cfg(test)` so the harness selects frames by name, or is the byte-level harness preferred as deliberate independence from the roster? Recommendation: re-export; the roster snapshot already owns the numbers.
- The "lie"/"deceived" vocabulary is crate-wide (43 sites outside this partition, including the `GreetingLie` test type). Recommendation: one prose commit sweeping to the "mis-declared" register 408ede87 chose, with the type renamed in the same pass.
- The blind-spots lens notes that `symmetric_accepts_with_distinct_payloads_are_live` is the only proxy-tier test with non-unit payloads, and no multi-record run under transport adversity carries a non-unit payload. Recommendation: low priority; payload identity is pinned end to end by the public suites and at the codec tier; revisit if a payload-parameterised generator lands for other reasons.
- The seed file `proptest-regressions/tree/mirror/streaming/remote/proxy/tests.txt` records shrinks under the generators of their day; the blind-spots lens believes two entries predate the `schedule` draw. Tuple draws are prefix-stable so the seeds still replay. Recommendation: leave them; seeds are proptest's, and the comments are informational.

## Dropped

- [15] Fault-surface roster hand-enumerated where the proptest derives it by rule: refuted. The proptest also hand-enumerates its five operations in `prop_oneof!`; the deterministic list is an expectation list of the sanctioned tamper-evident form, and `completed()` matches `IoOperation` exhaustively so a new operation fails to compile in this file.
- [44] `PayloadCodec::new::<T>(PayloadDepthLimit::default())` 67 times: folded into remote-proxy-tests-5 (the topology consolidation leaves one site here; a `pub(crate)` convenience on `PayloadCodec` is a crate-wide question outside the partition).
- [2] n-versus-m fixtures, [3] duplicated helpers, [41] election predicate spelled twice: merged into remote-proxy-tests-4.
- [22], [37] greeting test names: duplicates of remote-proxy-tests-1.
- [24], [33] reordered wide property: duplicates of remote-proxy-tests-6.
- [26], [29] join witness placement: duplicates of remote-proxy-tests-20.
- [27] duplicated helpers and magic capacities: duplicate of remote-proxy-tests-4, -17, and -23.
- [28] test docs wider than assertions: duplicate of remote-proxy-tests-11 and -13.
- [30] Merkle-hash comparison: duplicate of remote-proxy-tests-15.
- [31] topology hand-rolled: duplicate of remote-proxy-tests-5; its sub-claim that the two alias families "name the same shape" is corrected (they differ in backend error type, `Failure<Infallible>` versus `Infallible`).
- [32] endpoint projection: duplicate of remote-proxy-tests-16.
- [21], [36] state codes: duplicates of remote-proxy-tests-21, whose resolution takes [36]'s re-export.
- [43] unexplained capacities: duplicate of remote-proxy-tests-17.
- [45] work/tests.rs module doc and `31`: duplicate of remote-proxy-tests-13 and -17.
- Blind-spots open question on `ScriptedConnector` preserving the inner `Done`: below the bar today (the memory connector returns `Done::discard()` itself); the captured-`Done` mechanism it would enable is what remote-proxy-tests-25 asks for.
