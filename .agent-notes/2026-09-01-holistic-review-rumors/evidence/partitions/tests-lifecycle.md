# Partition tests-lifecycle: Integration tests: bootstrap, retire, lifecycle, membership, multi-peer, pairwise, partition, reuse, redaction

## Partition summary

This partition is the crate's public-API behavioral suite for the replica lifecycle, at commit 9e5784fb. Fifteen integration binaries (3171 lines, all test code) cover: bootstrap over the wire and every arm of the bookmarked builder's `Joined` outcome (`tests/bootstrap.rs`), with byte pins in `tests/bootstrap_snapshot.rs`; retirement outcomes, content survival, and a plain-gossip differential oracle (`tests/retire.rs`, `tests/retire_redaction.rs`), with byte pins in `tests/retire_snapshot.rs`; the session promise, link poisoning, and the post-commit `Epilogue` residue (`tests/lifecycle.rs`); connection reuse and the epoch wrap (`tests/reuse.rs`); the per-universe `Network` guard (`tests/network.rs`); single-peer batch semantics (`tests/single_peer.rs`); the algebraic laws of one gossip session (`tests/pairwise.rs`); redaction corners (`tests/redaction.rs`); and the schedule-executor family checked against the spec-shaped oracle (`tests/multi_peer.rs`, `tests/partition.rs`, `tests/membership.rs`, `tests/sanity.rs`). To settle the findings I also read the harness these suites stand on (`tests/common/wire.rs`, `action.rs`, `peer.rs`, `oracle.rs`, `mod.rs`, `window.rs`, `tests/async_wire.rs`, about 890 lines, plus cited ranges of the schedule executor, the shadow generator, `flaky.rs`, and `sim.rs`) and the crate sources the testdocs cite.

The suite is in good shape. Every session runs over in-memory links under the closed-world poller (`common::wire::block_on` is `run_to_quiescence`), so a protocol stall fails deterministically at the offending poll. Every successful session in the partition ends in `assert_control_drained`, and the assert carries its own committed negative control. Several tests assert their own premises (the zero-stream check in `reuse.rs`, the budget placement in `lifecycle.rs`, the empty-store precondition in `bootstrap.rs`), and `membership.rs` pins the liveness of its generated dimension. The oracle is pure data that never invokes the merge. Every test carries an English testdoc. No residue of the V1 protocol, BLAKE3, or the height and item erasure appears in these files; there are no ignored tests, no commented-out code, and no debug prints.

The dominant issues are prose written on one side of a design boundary and carried across it unchanged, and harness plumbing re-spelled per binary. Four boundaries explain nearly every stale sentence: retire reconciling before it relinquishes the party (fedb3ecb2), the callback API's removal and the shared-state port (db32b94d9, 1d3a3df4d1), the `rumors::sync` surface's removal (83edcd944), and the per-stream link (b3b877d9bf). The concrete casualties are a `retire.rs` header and testdocs describing a domination precondition and declines the `Retire` contract does not have, a `partition.rs` module doc naming an `on_message` callback and a `key` that exist nowhere in the crate, a `redaction.rs` testdoc claiming the public docs are silent where `Rumors::redact` speaks, a `single_peer.rs` testdoc citing a batch-docs promise that was deleted, and `async_known` plus a section header still qualified against a sync surface that is gone. On the structural side, the "serve a bootstrap" and "retire over a link" sessions are written out ten times across the partition and its neighbors instead of once in `common::wire`, and one copy has already drifted (the bookmarked join and the mutual bookmarked bail skip the drain check every other driver runs). Three verification gaps are worth scheduling: nothing pins that redactions stay in the generated populations (the membership alphabet has exactly this floor; the redaction dimension does not), two `redaction.rs` tests assert quantities a redaction cannot change while leaving the documented no-op contract unpinned, and the epoch-wrap test never reads the public `SessionState::epoch()` it exists to exercise.

## Findings

### tests-lifecycle-1: A committed seed annotated for a property that no longer exists
- Where: proptest-regressions/retire.txt:8-8 (related: tests/retire.rs:352-443, tests/seed_liveness.rs:207-242)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 1d3a3df4d -- tests/retire.rs` removes `fn retire_refused_iff_snapshots_outstanding(n in 0usize..4)` at diff line 658; both live properties in retire.rs take `(a_actions, b_actions)`; `tests/seed_liveness.rs:207-242` read in full, it checks path resolution only)
- Seen by: blind-spots; refutation: confirmed; history: deliberate-but-expired (the seed was committed in 89e447841 for a property deleted the next day; it survived the port and the 2026-09-01 relocation because the never-strip rule kept it out of every diff)
- Owner-gated: yes: AGENTS.md's never-strip rule makes any edit to a seed file an owner call

The second `cc` line records a shrunk counterexample `n = 1` for a property `tests/retire.rs` no longer contains; the header at retire.rs:16-18 states that case is unrepresentable. The line still replays as an RNG seed for the two live properties (harmless), but its annotation refers to code that does not exist, and `seed_liveness.rs` cannot see it because it checks that seed files resolve to live source paths, not that each `cc` line's parameters match a live property. The never-strip rule exists to preserve regression coverage; this line preserves none.

Evidence:

         8	cc f63e981a4e7979519ac37ccb1ac2e1f16d85925cb8b50228079c1d673e708005 # shrinks to n = 1

Resolution: Remove the line in a commit whose message names the deleted property, or record in the commit that it is retained deliberately. Consider extending `seed_liveness.rs` to parse each `cc` comment's parameter names against the live `proptest!` signatures in the owning file, so a deleted property's seeds surface mechanically. Acceptance: retire.txt carries only `cc` lines whose parameter names match a live property in tests/retire.rs, or the retention is recorded; if the sweep is extended, a fixture with a mismatched `# shrinks to` fails it.

### tests-lifecycle-2: Three partition-local 64 KiB `LINK_BUF` constants with rationales the harness does not support
- Where: tests/bootstrap.rs:24-27 (related: tests/retire.rs:31-33, tests/reuse.rs:28-31, tests/party_conservation.rs:48-51, tests/common/wire.rs:61-63, tests/common/wire.rs:248, tests/common/schedule/executor.rs:258)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'const LINK_BUF' tests/` lists ten definitions, four at 8 KiB including `common::wire::LINK_BUF`, six at 64 KiB; wire.rs:248 serves every harness bootstrap at 8 KiB; executor.rs:258 runs every executor retire session at 8 KiB)
- Seen by: structure-prose, api-economics; refutation: reframed (bootstrap.rs does not claim 8 KiB fails; its choice is unmotivated, and retire.rs and party_conservation.rs justify 64 KiB by pointing at each other); history: deliberate-but-expired (the 64 KiB constants and their prose were written for the single duplex pipe under the mux; b3b877d9bf renamed `DUPLEX_BUF` to `LINK_BUF` and carried the sentences across unchanged)
- Owner-gated: no

The bootstrap.rs constant is justified by keeping "the bootstrap descent's largest frames" clear of backpressure, yet `bootstrap_fork` serves every harness bootstrap over the 8 KiB `common::wire::LINK_BUF`, whose own doc states the opposite policy ("A modest buffer is sufficient and naturally exercises per-stream backpressure"). retire.rs:31-33 keeps 64 KiB for "the other wire tests' headroom" while party_conservation.rs:48-51 keeps it to "keep `retire.rs`'s headroom": a circular chain, and the executor runs the same retire sessions at 8 KiB. Only reuse.rs:28-31 states a reason local to its test shape (an eager side must write its next preamble without waiting on the laggard). Prose must describe today's code; a rationale the tree elsewhere falsifies misleads the next person sizing a buffer.

Evidence:

        24	/// Capacity for each in-memory link stream. Roomy enough that the bootstrap
        25	/// descent's largest frames fit without the test depending on backpressure
        26	/// subtleties.
        27	const LINK_BUF: usize = 64 * 1024;

        31	/// Capacity for each in-memory link stream. A divergent retiree's session moves
        32	/// content through the gossip round, so keep the other wire tests' headroom.
        33	const LINK_BUF: usize = 64 * 1024;

Resolution: Import `crate::common::wire::LINK_BUF` in bootstrap.rs and retire.rs (and party_conservation.rs, whose comment dangles once retire.rs changes). Keep reuse.rs's constant with its shape-specific rationale, or promote it to wire.rs under a name that carries that rationale. Acceptance: `grep -rn 'const LINK_BUF' tests/` returns wire.rs plus only definitions whose comment names a test shape a smaller buffer would change; bootstrap, retire, and party_conservation pass unchanged.

### tests-lifecycle-3: The bootstrap and retire session drivers are spelled out per suite instead of owned by `common::wire`
- Where: tests/bootstrap.rs:31-49 (related: tests/bootstrap.rs:165-180, tests/bootstrap.rs:209-224, tests/network.rs:87-99, tests/retire.rs:52-65, tests/retire.rs:78-87, tests/retire.rs:91-110, tests/retire_redaction.rs:39-46, tests/common/wire.rs:244-264, tests/common/wire.rs:232-240, tests/party_conservation.rs:103-120, tests/common/schedule/executor.rs:256-275)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read `wire_bootstrap` and `bootstrap_fork_configured` side by side: same link pair, same `tokio::join!` of `gossip` and `bootstrap().join`, same `expect`, same floor, same drain; the only differences are the 64 KiB buffer and an `Option` return that both callers `.expect` at lines 68 and 101; each other listed site read and confirmed to carry the same shape)
- Seen by: structure-prose, api-economics; refutation: reframed (the 64 KiB buffer is an unmotivated policy deviation, not a contradicted claim) and confirmed for the roster, adding party_conservation.rs:103-120; history: no-rationale-found (the per-suite drivers predate the shared helper's current shape and were never folded in)
- Owner-gated: no

`wire_bootstrap` is `bootstrap_fork_configured(provider, WindowChoice::Floor)` with a different buffer and an `Option` return no caller uses as an `Option`. The same provider-gossip/newcomer-join session is written inline again at bootstrap.rs:165-180 and network.rs:87-99 and as `wire_bookmarked_join` at 209-224; the retire-into-gossip session is written as three harnesses in retire.rs, inline in retire_redaction.rs, inline in party_conservation.rs, and inline in the executor. Each copy differs only in the builder configuration or the outcome type. A helper every suite reimplements is a missing helper, and the copies have already drifted: the bookmarked variant omits the drain check (tests-lifecycle-7). A future change to the session-boundary contract would need ten edits.

Evidence:

        31	fn wire_bootstrap<T>(provider: &Rumors<T>) -> Option<Rumors<T>>
        32	where
        33	    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
        34	{
        35	    block_on(async move {
        36	        let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);
        37	
        38	        let (provider_out, bootstrap_out) = tokio::join!(
        39	            provider.gossip(&mut a_link),
        40	            Peer::<T>::bootstrap().join(&mut b_link),
        41	        );
        42	        provider_out.expect("provider gossip");
        43	        let joined = bootstrap_out
        44	            .expect("bootstrap handshake")
        45	            .map(|peer| peer.sync_window_floor().into_rumors());
        46	        assert_control_drained(a_link, b_link);
        47	        joined
        48	    })
        49	}

Resolution: In `tests/common/wire.rs` add a bootstrap driver that takes the configured `Bootstrap<T>` builder (so the zero-budget and bookmarked variants collapse into the caller's builder chain), a `Joined`-returning sibling for the bookmarked builder, and a `retire_into_async(retiree: Peer<T>, absorber: &Rumors<T>) -> Retire<T>`, each ending in `assert_control_drained`; express `bootstrap_fork_configured` through the first. Replace `wire_bootstrap(&provider).expect(..)` with `bootstrap_fork(&provider)`, and the inline sessions with the helpers. `bootstrap_fork_with_window_async` (wire.rs:232-240) is a pure pass-through of `bootstrap_fork_configured`; give the underlying function the public name. Acceptance: `grep -n 'bootstrap().*join(&mut' tests/*.rs` matches only wire.rs and tests whose subject is the builder itself; `grep -n '\.retire(&mut' tests/*.rs tests/common` matches only wire.rs and the snapshot capture drivers; no `fn wire_bootstrap` and no local `LINK_BUF` in bootstrap.rs; all affected suites pass.

### tests-lifecycle-4: The bootstrap testdoc claims a failure mode the body cannot detect
- Where: tests/bootstrap.rs:52-59 (related: tests/bootstrap.rs:79-87, tests/bootstrap.rs:90-118, tests/party_conservation.rs:237-262, tests/stale_floor.rs:1-20)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read; reasoning over the body at 79-87: the provider sends nothing after the fork, so a newcomer ticking a copied party still lands strictly above the provider's ceiling and its message is learned, not dropped)
- Seen by: blind-spots, structure-prose (the "silently destroy" tell); refutation: confirmed; history: no-rationale-found (the wording is original to the shared-state port)
- Owner-gated: no

The doc says the newcomer's origination surviving gossip back into the provider is something "a non-disjoint or stale-floored party would silently destroy." The stale-floor half holds (and names its mechanism in `tests/stale_floor.rs`). The non-disjoint half does not: a copied party manifests only when both sides originate concurrently (same version, two payloads), and the provider originates nothing after the fork here. Disjointness is pinned directly in `tests/party_conservation.rs:237-262`, so the invariant is covered; this doc is inaccurate about what this test proves, and "silently destroy" appears without the mechanism (a dominated version reads as already forgotten). An inaccurate testdoc is a bug in the test.

Evidence:

        52	    /// Bootstrapping from a provider yields exactly the provider's live
        53	    /// content, message identities included (versions are stable across
        54	    /// peers), leaves the provider's own content untouched, and creates a
        55	    /// *disjoint* party.
        56	    ///
        57	    /// Disjointness is proven behaviorally: a message the newcomer originates
        58	    /// survives a gossip round back into the provider, which a non-disjoint or
        59	    /// stale-floored party would silently destroy.

Resolution: Either drop "non-disjoint" and "creates a *disjoint* party" from the doc, cite `party_conservation.rs` for disjointness, and state the floor mechanism ("a stale floor would leave the newcomer's version dominated, and deletion honoring would drop it as already seen"); or sharpen the body so the claim is true: have the provider also `send` after the fork, gossip, and assert both payloads live on both sides. Apply the same to the `String` variant. Acceptance: the doc's stated failure mode is one the body demonstrably detects, and the adverb has its mechanism beside it.

### tests-lifecycle-5: No point pin that a newcomer's inherited floor suppresses a stale peer's copy of a redacted message
- Where: tests/bootstrap.rs:79-87 (related: tests/stale_floor.rs:27-40, tests/common/schedule/arb.rs:288-295, tests/common/schedule/arb.rs:315-336, src/rumors.rs:182-194)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read; `grep 'resurrect' tests/` finds only the two-peer gossip_snapshot cases and an unrelated bookmark_causality mechanism; the membership shadow's `record_bootstrap` copies `ever_known` and `absorb` honors `dst_known && !dst_live`, so the shape is reached generatively in membership.rs only)
- Seen by: blind-spots; refutation: confirmed; history: no-rationale-found (`stale_floor.rs` was born as a reproducer for one direction of the floor's job)
- Owner-gated: no

The comment states the newcomer is floored at the served tree's version, and `stale_floor.rs` pins one consequence: a newcomer's own sends survive. The other half of the floor's job has no point test: a newcomer served after the provider redacted X must, on first gossip with a third peer that still holds X, treat X as deleted rather than re-learn it. A bootstrap that served the tree with a floor below the redaction's tick would pass every test in this file and resurrect redacted content at the first three-peer gossip; only the membership engine reaches the shape, generatively.

Evidence:

        79	        // The newcomer's party is disjoint from the provider's retained half
        80	        // and floored at the served tree's version, so a fresh origination
        81	        // survives reconciliation on both sides.

Resolution: Add a point test beside `bootstrap_reproduces_a_fork`: seed sends X; `stale = bootstrap_fork(&seed)`; seed redacts X; `newcomer = bootstrap_fork(&seed)`; `wire_gossip(&newcomer, &stale)`; assert X absent from both snapshots and the newcomer's `latest()` dominates X's version. Acceptance: the test fails if the bootstrap serves a floor below the redaction tick instead of the served tree's ceiling.
Construction: `seed.send(X); let stale = bootstrap_fork(&seed); seed.redact(&v_x); let newcomer = bootstrap_fork(&seed); wire_gossip(&newcomer, &stale); assert!(newcomer.snapshot().get(&v_x).is_none() && stale.snapshot().get(&v_x).is_none())`. With `Rumors::redact`'s documented deletion honoring this passes today; a floor regression makes it fail.

### tests-lifecycle-6: Twin proptest bodies and three local pair builders that a shared generic body or a `common` helper would own
- Where: tests/bootstrap.rs:90-94 (related: tests/bootstrap.rs:60-88, tests/lifecycle.rs:50-58, tests/reuse.rs:52-56, tests/gossip_when.rs:61-65, tests/common/wire.rs:175-187, tests/multi_peer.rs:182-200)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (`diff` of bootstrap.rs:62-88 against 95-118 with `u64`/`String` normalized differs only in the sentinel payload and the body comment; `grep 'fn pair\b\|fn divergent_pair'` lists lifecycle.rs:50, reuse.rs:52, gossip_when.rs:61, wire.rs:175)
- Seen by: structure-prose, api-economics; refutation: confirmed (adds gossip_when.rs's third `pair()`); history: no-rationale-found
- Owner-gated: no

The two bootstrap proptests differ only in `T`, the strategy, and the sentinel; `wire_bootstrap` is already generic, so a `fn check_bootstrap_reproduces<T>(actions, sentinel) -> Result<(), TestCaseError>` leaves two three-line entries. Separately, lifecycle.rs defines `divergent_pair()` with the same name as `common::wire::divergent_pair(values_per_side, window_a, window_b)` but a different topology (seed plus one fork versus two forks of a seed), and reuse.rs and gossip_when.rs both define `pair()` as lifecycle's first two lines. A local name identical to a `common` helper's makes the reader check which is in scope.

Evidence:

        90	    /// `String`-`T` variant of [`bootstrap_reproduces_a_fork`]: the same
        91	    /// invariant for a non-primitive value type, exercising the wire
        92	    /// round-trip of the whole-tree frame for `T = String`.
        93	    #[test]
        94	    fn bootstrap_reproduces_a_fork_string(actions in arb_string_actions()) {

Resolution: Extract the bootstrap body as a generic fn called from both entries (multi_peer.rs:182-200 has the same twinning at smaller scale). Add `pub async fn seeded_pair_async<T>() -> (Rumors<T>, Rumors<T>)` to `common::wire`, use it in lifecycle.rs, reuse.rs, and gossip_when.rs, and rename lifecycle's builder (`deep_divergent_pair`). Acceptance: the two bootstrap proptests share one body; no two fns named `divergent_pair` in tests/; no `fn pair` in reuse.rs or gossip_when.rs.

### tests-lifecycle-7: The bookmarked join driver and the mutual bookmarked bail skip the drain check every other successful session runs
- Where: tests/bootstrap.rs:209-224 (related: tests/bootstrap.rs:285-295, tests/bootstrap.rs:46, tests/bootstrap.rs:136, tests/bootstrap.rs:178, tests/common/wire.rs:65-79)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -n 'assert_control_drained\|tokio::join!' tests/bootstrap.rs`: joins at 38, 132, 167, 215, 287; drains at 46, 136, 178 only)
- Seen by: api-economics; refutation: confirmed; history: no-rationale-found (the helper landed six days after the drain gate was applied "one call per site"; the introducing commit does not mention drain)
- Owner-gated: no

`wire_bookmarked_join` returns the `Joined` outcome without calling `assert_control_drained`, and `mutual_bookmarked_bail_returns_the_bookmark` runs its mutual bootstrap at 285-295 without one either, whereas `both_bootstrapping_bail_with_none` (136) drains the identical unbookmarked session. `Joined::Joined` and `Joined::Bailed` are successful sessions under the assert's contract (wire.rs:65-79), so the bookmarked join path's boundary cleanliness is unchecked by the four `Joined`-arm tests that use the helper. The drain invariant was gated because a leftover control byte surfaces later as a confusing violation; a driver that omits the gate leaves that class uncaught on its path. Totality over spot checks.

Evidence:

       209	fn wire_bookmarked_join(
       210	    provider: &Rumors<u64>,
       211	    bookmark: FlakyInMemoryBookmark,
       212	) -> Joined<u64, FlakyInMemoryBookmark> {
       213	    block_on(async move {
       214	        let (mut provider_link, mut newcomer_link) = rumors::link::memory_with_capacity(LINK_BUF);
       215	        let (served, joined) = tokio::join!(
       216	            provider.gossip(&mut provider_link),
       217	            Peer::<u64>::bootstrap()
       218	                .bookmark(bookmark)
       219	                .join(&mut newcomer_link),
       220	        );
       221	        served.expect("the provider serves the bookmarked bootstrap");
       222	        joined
       223	    })
       224	}

Resolution: Inside the helper, call `assert_control_drained(provider_link, newcomer_link)` for `Joined::Joined`, `Joined::Bailed`, and `Joined::Unbookmarked` (the persist failure happens after the session, so its boundary is clean too), skipping only `Joined::Failed` with a one-line reason; add the drain to the mutual bail at 285-295. Both disappear under the shared driver of tests-lifecycle-3. Acceptance: every successful bookmarked join and bail in bootstrap.rs passes through `assert_control_drained`; the planted-byte negative control in reuse.rs still fails the assert.

### tests-lifecycle-8: `seeded()` is copied into four binaries, one copy documenting the duplication
- Where: tests/bootstrap_snapshot.rs:33-43 (related: tests/retire_snapshot.rs:35-44, tests/network.rs:16-22, tests/gossip_snapshot.rs:31, tests/opening_supply.rs:25-27)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'fn seeded' tests/`; opening_supply.rs:27 omits `sync_window_floor`, so it is a different fixture; observe.rs:258 takes an observer and is distinct)
- Seen by: structure-prose, api-economics; refutation: reframed (four identical copies modulo the stream parameter; opening_supply's is a distinct fixture); history: no-rationale-found (the "Mirrors" sentence was added in a style pass that documented the duplication instead of removing it)
- Owner-gated: no

bootstrap_snapshot.rs, retire_snapshot.rs, network.rs (parameterized by stream), and gossip_snapshot.rs each define the fixed-RNG seed at the floor; the bootstrap_snapshot copy's doc says "Mirrors `gossip_snapshot::seeded`", naming the duplication rather than removing it. The copies diverge in trivia (`SeedableRng as _` versus `SeedableRng`; fully qualified serde bounds beside imports of the same names three lines up). One helper in `common` is one place to change when `seed_rng`'s signature or the floor policy moves.

Evidence:

        33	/// A provider seeded from a fixed RNG, so the [`rumors::Network`] id carried in
        34	/// the preamble — and the party region it forks off for the newcomer — are
        35	/// deterministic and these captures stay reproducible.
        36	///
        37	/// Mirrors `gossip_snapshot::seeded`.
        38	fn seeded<T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static>()
        39	-> Rumors<T> {
        40	    Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
        41	        .sync_window_floor()
        42	        .into_rumors()
        43	}

Resolution: Add `pub fn seeded_floor<T>(stream: u64) -> Rumors<T>` to `common::wire` (network.rs's shape subsumes the zero-stream callers); delete the four copies and the "Mirrors" sentence; leave opening_supply's default-window fixture alone or give the helper a `WindowChoice`. Acceptance: `grep -rn 'fn seeded' tests/*.rs` returns only helpers with a distinct signature; the `.snap` files are byte-identical (the fixture is unchanged).

### tests-lifecycle-9: multi_peer runs four executor passes per case for properties one pass proves, two of them implied by a third
- Where: tests/multi_peer.rs:40-100 (related: tests/multi_peer.rs:106-124, tests/multi_peer.rs:168-200, tests/membership.rs:60-70, tests/common/peer.rs:78-87, tests/redaction.rs:3-5)
- Class / severity / confidence: performance / low / high
- Provenance: verified (all four properties draw `(schedule_u64(), arb_window_assignment())` and call `execute_and_quiesce` at lines 44, 62, 85, 110; `diff` of membership.rs:53-58 against multi_peer.rs:86-91 is identical modulo the binding name)
- Seen by: api-economics; refutation: confirmed with a caveat (the multiset check is implied by the canonical-map check only while `resolved_versions` is injective, which the harness's `insert_one` assert at peer.rs:85 enforces) and downgraded to low; history: deliberate-and-holds (80a3155f41 chose one named test per invariant "for auditability"; the rationale lives only in that commit message and never weighed the executor cost or the implication)
- Owner-gated: yes: reopens a recorded design choice (one named test per invariant)

If every peer's `readout` equals the canonical map (`versions_stable_across_peers`), then all readouts are pairwise equal (`all_peers_converge_after_quiesce`) and the value multiset of the canonical map is exactly `expected_live()` (`readout_matches_oracle_after_quiesce`), given that `resolved_versions` is injective, which `Peer::insert_one`'s exactly-one-new-observation assert enforces. The two implied tests therefore add no sampled space at 512 executor-plus-quiesce runs (up to 8 peers, 50 events, 256 cases each). The recorded rationale (each invariant named after what it tests) is a legibility argument and still holds; it is not stated in the module, and it never considered fusing assertions over one result with distinct messages.

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

        81	    fn versions_stable_across_peers(
        82	        schedule in schedule_u64(),
        83	        windows in arb_window_assignment(),
        84	    ) {
        85	        let result = execute_and_quiesce(&schedule, &windows);

Resolution: Owner's choice between (a) fusing: delete the two implied tests, fold the observation-log check into the canonical-map test over the same `result` with distinct assertion messages, switch the `_at_floor` and `_string` legs to the canonical-map form (stronger at equal cost), and note beside the fused test that the harness's `insert_one` assert carries the injectivity premise; or (b) keeping the split and stating the one-test-per-invariant policy in the module doc so the cost reads as chosen. Either way, redaction.rs:3-5 and multi_peer.rs:182 name `readout_matches_oracle_after_quiesce` and must follow the rename. Apply the same reasoning to membership.rs:62-69, where the multiset assert is implied by the canonical one that follows it. Acceptance: for (a), `execute_and_quiesce` runs at most once per generated `(schedule, windows)` case in the u64 swept leg and every property the deleted tests held is asserted in a survivor; for (b), the module doc states the policy.

### tests-lifecycle-10: `Peer::seed_rng` is public but hidden and ungated, while the partition depends on it for reproducibility
- Where: tests/network.rs:16-22 (related: tests/bootstrap_snapshot.rs:38-43, tests/retire_snapshot.rs:40-44, src/peer.rs:210-213, src/peer.rs:480-483, src/peer.rs:713-716, src/rumors.rs:413-416, src/network.rs:62)
- Class / severity / confidence: api-surprise / low / medium
- Provenance: verified (src/peer.rs:212-213 `#[doc(hidden)] pub fn seed_rng` with no `cfg`; `sync_window_floor` at 480-483 and `dangerously_alias_party` at 726-728 are gated on `test-internals`; `Network::from_rng` is `pub(crate)`; `warm_caches` at peer.rs:713 and rumors.rs:413 has the same hidden-ungated shape)
- Seen by: api-economics; refutation: confirmed (adds `warm_caches`); history: no-rationale-found (documented public API with a doctest at introduction, hidden two days later in a WIP commit whose message is silent on it)
- Owner-gated: yes: the resolution either documents a public method or gates it

The snapshot suites and network.rs reach `Peer::seed_rng` to get a deterministic `Network`. A user writing replay or snapshot tests of an application built on rumors has exactly this need and cannot discover the method; meanwhile it is semver-visible surface with no documentation contract, unlike `sync_window_floor`, which is gated. Hidden-but-public binds the crate to the signature without telling users it exists. `warm_caches` ("For benchmark and test calibration only") has the same shape, and dev-dependencies already enable `test-internals` for benches and tests (Cargo.toml:145).

Evidence:

        16	/// A peer seeded deterministically, so two seeds with distinct stream ids get
        17	/// distinct (but reproducible) networks.
        18	fn seeded<T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static>(
        19	    stream: u64,
        20	) -> Peer<T> {
        21	    Peer::seed_rng(&mut SmallRng::seed_from_u64(stream)).sync_window_floor()
        22	}

Resolution: Owner decision covering both methods: (a) un-hide `seed_rng` with a short doc stating the reproducibility use and the one hazard (two universes seeded from equal RNG state share a `Network` and must never interact), or (b) add `#[cfg(any(test, feature = "test-internals"))]` beside the existing `#[doc(hidden)]` on `seed_rng` and `warm_caches`. Acceptance: each is either documented in the rendered rustdoc or unreachable from a default build.

### tests-lifecycle-11: The network-mismatch test uses empty peers, so its "before any content crosses" claim is vacuous, and the retire dual is untested
- Where: tests/network.rs:56-77 (related: src/peer/gossip.rs:1155-1164, src/peer/gossip.rs:696-702, src/peer/gossip.rs:1384-1404, tests/bookmark_causality.rs)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rln NetworkMismatch tests/ src/`: in tests only network.rs and bookmark_causality.rs, both on the gossip path; gossip.rs:1157-1163 places the check after the handshake and before `reconcile()` at 1164; gossip.rs:696-702 takes the whole party when retiring, before that check, and `PartyGuard::drop` at 1398-1400 restores it)
- Seen by: blind-spots; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`gossip_rejects_foreign_network` seeds two universes and gossips them with nothing sent on either side. The doc says rejection happens "before any content crosses the wire", but with empty replicas there is no content to cross, and the session promise that `Err` leaves the replica unchanged goes unchecked. Production checks `remote_network != network` before `reconcile()`, so the claim is true today, but nothing holds it. The dual is also missing: a retiree from universe X retiring into an absorber from universe Y takes its whole party speculatively before the check and relies on `PartyGuard` to restore it; no test in tests/ exercises `NetworkMismatch` on the retire path. Two independently seeded universes must never interact; this guard is the only thing between them.

Evidence:

        56	/// Two peers from different seeds that try to [`gossip`](rumors::Rumors::gossip)
        57	/// are both rejected with [`Error::NetworkMismatch`] at the handshake, before
        58	/// any content crosses the wire.
        59	#[test]
        60	fn gossip_rejects_foreign_network() {
        61	    let alice = seeded::<u64>(1).into_rumors();
        62	    let bob = seeded::<u64>(2).into_rumors();

Resolution: Populate both peers with distinct content and take hashes before the session; wrap both links with `rumors::testing::wrap_link` and assert `connects + accepts == 0` on both reports and both hashes unchanged after the `NetworkMismatch`. Add `retire_rejects_foreign_network`: convert a populated universe-X handle to a `Peer`, retire it against a universe-Y gossiper, assert `Retire::Recovered { error: Error::NetworkMismatch { .. }, peer }`, the absorber's `Err(NetworkMismatch)`, the recovered peer's hash unchanged, and (via `dangerously_alias_party`) its party equal to the pre-session alias. Acceptance: moving the network check after `reconcile()` fails the sharpened gossip test on the stream counters; dropping the `PartyGuard` restore fails the retire test on the party comparison.
Construction: for the retire dual, `let x = seeded::<u64>(1).into_rumors(); x.send(1)?; let y = seeded::<u64>(2).into_rumors(); let pre = x.dangerously_alias_party(); let x = block_on(x.try_into_peer())?; let (out, served) = block_on(async { tokio::join!(x.retire(&mut lx), y.gossip(&mut ly)) });` then match `out` against `Retire::Recovered { error: Error::NetworkMismatch { .. }, peer }` and compare `peer.into_rumors().dangerously_alias_party()` to `pre`.

### tests-lifecycle-12: Import idioms: qualified paths beside imports that exist, serde import lines with no separating blank, and a `futures` no-op waker beside std's
- Where: tests/pairwise.rs:22-55 (related: tests/pairwise.rs:72, tests/pairwise.rs:94, tests/pairwise.rs:125, tests/pairwise.rs:148, tests/pairwise.rs:165, tests/pairwise.rs:189, tests/pairwise.rs:224, tests/multi_peer.rs:142, tests/membership.rs:27, tests/multi_peer.rs:21, tests/partition.rs:30, tests/sanity.rs:15, tests/bootstrap_snapshot.rs:31-38, tests/network.rs:18, tests/bootstrap.rs:22-24, tests/single_peer.rs:16-18, tests/lifecycle.rs:21, tests/lifecycle.rs:74, tests/common/wire.rs:102, tests/pairwise.rs:145)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -c 'rumors::Peer::' tests/pairwise.rs` is 9, eight in code, with no `Peer` in the file's imports; `grep -rn 'std::ops::RangeInclusive' tests/*.rs` lists four partition files; `grep -A1 '^use serde::de::DeserializeOwned;'` shows a doc comment on the next line in bootstrap.rs, bootstrap_snapshot.rs, pairwise.rs, single_peer.rs; `grep -rn 'noop_waker\|Waker::noop'` shows lifecycle.rs on `futures::task::noop_waker_ref` and the harness on std `Waker::noop()`; toolchain 1.97.1)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no-rationale-found (the serde blocks are a mechanized sweep's insertion artifact; the qualified bounds were re-introduced a day later by a second sweep)
- Owner-gated: no

(a) pairwise.rs imports `rumors::{Rumors, Version, causally}` and then writes `rumors::Peer::<u64>::seed()` at eight sites with no collision to justify it; multi_peer.rs:142 writes `crate::common::peer::Peer<u64>` in a closure type; `std::ops::RangeInclusive<usize>` is qualified in four suites; bootstrap_snapshot.rs:38 and network.rs:18 spell `serde::Serialize + serde::de::DeserializeOwned` in bounds (bootstrap_snapshot.rs imports both at 31-32). In redaction.rs and sanity.rs the qualification is forced by the harness type `common::peer::Peer` sharing the name. (b) `use serde::Serialize;` / `use serde::de::DeserializeOwned;` sits after the grouped imports with no blank line before the next item at bootstrap.rs:22-24, bootstrap_snapshot.rs:31-33, pairwise.rs:28-30, single_peer.rs:16-18. (c) lifecycle.rs uses `futures::task::noop_waker_ref` where the harness uses std `Waker::noop()`. (d) pairwise.rs:145 puts "true" in scare quotes. Imports over long qualified paths except where the qualification informs; consistent idiom is what keeps sibling files scannable.

Evidence:

        22	use rumors::{Rumors, Version, causally};

        55	        let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();

Resolution: Import `Peer` in pairwise.rs and `RangeInclusive` in the four suites; import `common::peer::Peer` in multi_peer.rs; use the imported serde names in bounds; fold the serde lines into the external-crate import group with a blank line after; use `Waker::noop()` in lifecycle.rs; replace the scare quotes with "the merge's idempotence on itself". Owner's call whether to rename the harness `Peer` (for example `SimPeer`) to release the forced qualifications in redaction.rs and sanity.rs. Acceptance: `grep -n 'rumors::Peer::<u64>::seed' tests/pairwise.rs` is empty; `grep -n 'std::ops::RangeInclusive' tests/*.rs` is empty; each partition file's `use` section ends in a blank line; lifecycle.rs has no `futures::task` import.

### tests-lifecycle-13: Vestigial leftovers: a one-line alias of `bootstrap_fork`, bare brace blocks that scope nothing, and window configuration in a suite that never gossips
- Where: tests/pairwise.rs:30-37 (related: tests/pairwise.rs:13-17, tests/sanity.rs:46-55, tests/single_peer.rs:1, tests/single_peer.rs:34)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (`git blame -L 46,55 tests/sanity.rs`: braces at 46, 49, 52, 55 from 1d3a3df4d1, bodies rewritten by ce27df86ed and 212c6914b9; `grep -c sync_window_floor tests/single_peer.rs` is 13 and the file's three `gossip` hits are all prose; `git show 1d3a3df4d1^:tests/pairwise.rs` documents `dup` as a `sync_bootstrap_fork` that a party-sharing snapshot could not replace)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: deliberate-but-expired (the braces scoped a drop-committed `Batch` guard; `dup` distinguished a bootstrap fork from a party-sharing snapshot, which the shared-state port made unrepresentable; the `.sync_window_floor()` calls came from a uniform sweep whose purpose does not apply to a no-gossip suite)
- Owner-gated: no

`dup` is a one-line alias of `bootstrap_fork` whose module doc (13-17) already names the real helper; the two bare `{ .. }` blocks in sanity.rs each wrap a single `send_all` statement and scope nothing; single_peer.rs is "with no gossip" yet calls `.sync_window_floor()` at thirteen sites, a configuration that binds only in sessions and so signals a dependency the suite does not have. Machinery that outlives the constraint that justified it; a reader stops to ask what each is for.

Evidence:

        30	/// A genuine, party-disjoint copy of `k`'s content: a fresh originator that
        31	/// holds the same live messages but ticks its own party region.
        32	fn dup<T>(k: &Rumors<T>) -> Rumors<T>
        33	where
        34	    T: Clone + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
        35	{
        36	    bootstrap_fork(k)
        37	}

        46	        {
        47	            alice
        48	                .send_all(alice_values.iter().copied()).unwrap();
        49	        }

Resolution: Replace `dup(..)` with `bootstrap_fork(..)` and delete the fn; replace each brace block with the single statement; drop `.sync_window_floor()` from single_peer.rs. Acceptance: pairwise.rs has no `fn dup`; no bare block statements in `forked_gossip_matches_direct_gossip`; `grep -c sync_window_floor tests/single_peer.rs` is 0; all three suites pass.

### tests-lifecycle-14: The `(hash, latest)` fingerprint and the canonical identity-to-value map are re-implemented per suite instead of living in `common::oracle`
- Where: tests/pairwise.rs:39-45 (related: tests/multi_peer.rs:142-145, tests/retire.rs:126-127, tests/retire.rs:332-333, tests/retire.rs:378-379, tests/retire.rs:388-389, tests/retire.rs:420-421, tests/retire.rs:430-431, tests/common/peer.rs:149-152, tests/common/sim.rs:886-889, tests/membership.rs:53-58, tests/multi_peer.rs:86-91, tests/common/oracle.rs:63-100)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'hash(), .*latest().clone()' tests/` lists the sites; `diff` of membership.rs:53-58 against multi_peer.rs:86-91 is identical modulo the binding name; tests/common/oracle.rs read in full: it holds `readout`, `readout_multiset`, `version_key`, and no fingerprint or canonical-map lens)
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The `(hash, latest)` fingerprint is a fn here, a closure in multi_peer.rs and in the harness's `quiesce_refs` and `sim::quiesce`, and an inline tuple at six sites in retire.rs; its type is spelled twice (pairwise.rs:42, common/peer.rs:156). The canonical map built from `resolved_versions` filtered by the oracle's redaction set appears verbatim in membership.rs and multi_peer.rs. `common::oracle` exists to hold these lenses; re-spelling them means a change to what a fingerprint is (say, adding the ceiling) touches ten sites.

Evidence:

        39	/// The `(hash, latest)` fingerprint of a peer: equal fingerprints mean the
        40	/// same live content *and* the same causal frontier — gossip between two
        41	/// peers with equal fingerprints is a guaranteed no-op.
        42	fn fingerprint<T>(k: &Rumors<T>) -> ([u8; rumors::MERKLE_HASH_LEN], Version) {
        43	    let snapshot = k.snapshot();
        44	    (snapshot.hash(), snapshot.latest().clone())
        45	}

Resolution: Add to `common::oracle` a `Fingerprint` type and `fn fingerprint<T>(&Snapshot<T>) -> Fingerprint`, and a `canonical_readout` on `ExecutionResult`/`MembershipExecutionResult` (or on `Oracle` taking `&resolved_versions`); replace the sites. Acceptance: `grep -rn 'hash(), .*latest().clone()' tests/` hits only common/oracle.rs; membership.rs and multi_peer.rs build the canonical map through one call.

### tests-lifecycle-15: Nothing pins that redactions stay in the `arb_local_actions` or `arb_schedule` populations
- Where: tests/pairwise.rs:47-53 (related: tests/multi_peer.rs:24-26, tests/partition.rs:48, tests/sanity.rs:25, tests/retire.rs:363-365, tests/bootstrap.rs:61, tests/membership.rs:99-135, tests/common/action.rs:26-32, tests/common/action.rs:79-83, tests/common/schedule/arb.rs:210-218, tests/common/schedule/arb.rs:365-372, tests/common/schedule/arb.rs:395-407)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -n 'Event::Redact\|LocalAction::Redact' tests/*.rs` returns nothing outside tests/common; `grep -rln 'TestRunner::deterministic' tests/` lists only membership.rs and window_sweep.rs; action.rs:79-83 drops `Redact` when no version exists, arb.rs:395-407 drops `RedactObservation` when the observed log is empty)
- Seen by: blind-spots; refutation: confirmed (committed seeds carry `Redact` actions, but a seed replays RNG state and a zero weight would regenerate insert-only cases); history: no-rationale-found (population pins were added for the membership alphabet and the budget arm as each landed; the older redaction dimension never received one)
- Owner-gated: no

`LocalAction::Redact(idx)` is dropped at build time when nothing has been sent, and `Choice::RedactObservation` is dropped when the peer's observed log is empty. Both drops are legitimate, but no test counts effectual redactions in a sampled population, so a `prop_oneof!` weight typo, a `%` bug in `lookup_observation`, or a change to `created_version` would leave every redaction-bearing property in pairwise, bootstrap, retire, async_wire, multi_peer, partition, and sanity green while exercising insert-only merges. `membership_population_contains_churn` pins exactly this liveness for the membership alphabet and says why ("Either could silently vanish ... leaving the suite green while testing no membership at all"). Meters need liveness floors: a property over a generated dimension passes vacuously when the generator stops producing it.

Evidence:

        47	proptest! {
        48	    /// After one bidirectional gossip session, the two peers' live
        49	    /// content (as exposed through `readout`) is equal.
        50	    #[test]
        51	    fn gossip_converges(
        52	        a_actions in arb_local_actions(),
        53	        b_actions in arb_local_actions(),

Resolution: Add deterministic-runner population tests mirroring `membership_population_contains_churn`: sample `arb_local_actions()` N times and assert some sample contains a `Redact` positioned after at least one `Insert`; sample `arb_schedule(any::<u64>(), N_PEERS, MAX_EVENTS)` and assert some emitted schedule contains an `Event::Redact`. Place them beside the strategies' consumers (pairwise.rs and multi_peer.rs) or in one `tests/population_liveness.rs`. Acceptance: setting the `Redact` weight to 0 in `arb_actions` or in `arb_choice` fails the new pin while the property suites stay otherwise green (the negative control demonstrates the pin fires).
Construction: in a scratch copy, change `1 => any::<usize>().prop_map(LocalAction::Redact)` at action.rs:29 to weight 0 and run pairwise, bootstrap, retire, async_wire: all green today; the proposed pin fails.

### tests-lifecycle-16: `gossip_order_independent`'s doc claims the union of all three; the body asserts only `a1 == a2`
- Where: tests/pairwise.rs:113-118 (related: tests/pairwise.rs:140, tests/pairwise.rs:212-237)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (line 140 is the body's only `prop_assert`; no expected union is built in 120-141)
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (doc and assertion were written together)
- Owner-gated: no

A testdoc states what the body checks, all of it and nothing more; "the union of all three" is a clause the test cannot back (`gossip_unions_content` checks the union for pairs).

Evidence:

       113	    /// Pairwise gossip is order-independent across three peers.
       114	    ///
       115	    /// Routing
       116	    /// everything through `a` first (`a·b` then `a·c`) and routing through
       117	    /// `b` first (`b·c` then `a·b`) both leave `a` holding the same
       118	    /// content — the union of all three.

Resolution: Either build `expected` from the three pre-gossip readouts with `extend` and assert it (as `gossip_unions_content` does), or trim the doc to "the same content". Acceptance: every clause of the testdoc corresponds to a `prop_assert`.

### tests-lifecycle-17: The union oracle's stated soundness premise omits the redaction exclusion, and the pairwise laws are pinned only on disjoint-content populations
- Where: tests/pairwise.rs:212-218 (related: tests/pairwise.rs:55-57, tests/pairwise.rs:72-74, tests/async_wire.rs:7-11, tests/common/action.rs:79-83, src/rumors.rs:451-452, tests/common/schedule/arb.rs:315-336, tests/common/schedule/arb.rs:395-403, tests/retire.rs:369-371, tests/retire.rs:412-414)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (action.rs:79-83: `Redact` only targets `versions[..]` the same peer inserted; every pairwise peer is `build_local(dup(&seed), ..)` over an empty seed; async_wire.rs:9-11 states the extra premise this doc omits; arb.rs:395-403 redacts from the observed log, which includes gossip-learned messages, so the schedule engines do reach the shared-redaction shape)
- Seen by: blind-spots; refutation: reframed (the shape is covered generatively by multi_peer's oracle under arbitrary gossip orders plus the fixed-point test; what remains is a doc inaccuracy and a narrower population for one file's laws) and downgraded to low; history: no-rationale-found
- Owner-gated: no

The doc justifies `BTreeMap::extend` as a union oracle by disjoint parties alone. That is necessary but not sufficient: the oracle is also sound only because `build_local` redacts only a peer's own pre-session sends, so no peer ever holds a message its counterparty redacted. Under the documented session promise (rumors.rs:451-452: both replicas hold every message either held "and neither had deleted"), the constructed scenario "seed sends X; a and b fork; a redacts X; gossip" makes the union oracle predict X live at both sides while the contract requires it absent. async_wire.rs:9-11 states the premise correctly. The same population shape means `gossip_converges`, `gossip_side_symmetric`, `gossip_idempotent`, and `gossip_order_independent` never see a redaction of shared content; the schedule engines cover that family, but this file's header does not say the division is deliberate. The doc also names `alice` / `bob` while the body's variables are `a` / `b`.

Evidence:

       212	    /// One session unions live content: after gossip, each side's readout
       213	    /// equals the union of the two pre-session readouts.
       214	    ///
       215	    /// The "union of readouts" is computed by `BTreeMap::extend`,
       216	    /// which is sound here only because readout keys are the leaf
       217	    /// versions' canonical bytes and `alice` / `bob` tick disjoint
       218	    /// parties, so they can't create the same version.

Resolution: State the full premise in the doc (disjoint parties, and redactions only of one's own pre-session sends), fix the names, and either state in the module header that shared-then-redacted content is the schedule engines' domain, or extend the population: draw `seed_actions` applied before the forks and let a post-fork redact phase target inherited versions, replacing the oracle with union minus every version either side redacted. Acceptance: the doc's stated reasons are sufficient for the oracle's soundness; the division of coverage between pairwise.rs and the engines is written where a reader of either will see it.

### tests-lifecycle-18: pairwise.rs and async_wire.rs pin the same union law under the same harness in two binaries
- Where: tests/pairwise.rs:220-237 (related: tests/pairwise.rs:51-61, tests/async_wire.rs:1-11, tests/async_wire.rs:43-59, tests/async_wire.rs:65-81, tests/bootstrap.rs:6-8)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`diff` of pairwise.rs:224-236 against async_wire.rs:47-58 with `dup`/`bootstrap_fork` and the `Peer` path normalized differs only in how `expected` is bound and in async_wire's extra fingerprint assert, which `gossip_converges` at 51-61 provides; both draw `arb_local_actions()` twice over an empty floored seed)
- Seen by: api-economics; refutation: confirmed; history: deliberate-but-expired (async_wire.rs existed to twin `sync_wire.rs`, and pairwise.rs dropped its wire tests then; the shared-state port re-pointed pairwise at the wire and 83edcd944 deleted the sync twin, leaving async_wire.rs without its reason)
- Owner-gated: no

`gossip_unions_content` together with `gossip_converges` is exactly `async_wire.rs::async_gossip_converges_on_the_union`: the same two `arb_local_actions()` inputs, the same construction, the same `extend` oracle, the same readout and fingerprint assertions, at default case counts in two link units. Only the `String` leg of async_wire.rs is unique. The framing "the *asynchronous* gossip path" (async_wire.rs:1, common/wire.rs:1) names a distinction the crate no longer has, and bootstrap.rs:6 says "Mirrors `async_wire.rs`'s setup".

Evidence:

       220	    fn gossip_unions_content(
       221	        a_actions in arb_local_actions(),
       222	        b_actions in arb_local_actions(),
       223	    ) {
       224	        let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();
       225	        let a = build_local(dup(&seed), &a_actions);
       226	        let b = build_local(dup(&seed), &b_actions);
       227	
       228	        let a_before = readout(&a.snapshot());
       229	        let b_before = readout(&b.snapshot());
       230	        let mut expected = a_before;
       231	        expected.extend(b_before);
       232	
       233	        wire_gossip(&a, &b);
       234	
       235	        prop_assert_eq!(readout(&a.snapshot()), expected.clone());
       236	        prop_assert_eq!(readout(&b.snapshot()), expected);

Resolution: Keep the union law in pairwise.rs (the algebraic-laws file), move the `String` leg there as one generic body called for both payload types, delete tests/async_wire.rs, and rewrite bootstrap.rs:6-8 to point at pairwise.rs or drop the sentence; drop "asynchronous" from common/wire.rs:1. Acceptance: one binary owns the union-of-readouts property for u64 and String; `grep -rl async_gossip_converges tests/` is empty; bootstrap.rs's module doc references no removed file.

### tests-lifecycle-19: partition.rs's module doc names an `on_message` callback and a `key` that exist nowhere in the crate
- Where: tests/partition.rs:9-19 (related: tests/shadow_validity.rs:15-16, tests/common/peer.rs:6-14, tests/common/schedule/executor.rs:95-100, tests/common/schedule/executor.rs:212-222)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn on_message src tests examples benches README.md AGENTS.md` hits only tests/partition.rs:12 and tests/shadow_validity.rs:16; `grep -rn 'pub struct Key\b\|pub type Key\b\|enum Key\b' src/` is empty; 9c73d7b463 is "api: retire Key; a message's public identity is its Version"; observation is pull-based through `Peer::drain` and the executor gates redacts on `peer.observations` at executor.rs:214)
- Seen by: structure-prose, blind-spots, api-economics (on_message); the `key` ghost is new in this pass; refutation: confirmed; history: contradicts-hard-rule (the callback API left the crate at db32b94d9 on 2026-06-10; 9c73d7b463 re-wrapped the sentence for the Key retirement on 2026-08-18 and kept both ghosts)
- Owner-gated: no

The argument for self-consistency over twin comparison is sound and worth keeping, but its referents are stale twice over: a redact is gated on the peer having "received the targeted message via an `on_message` callback", and a partitioned schedule may suppress redacts "because the targeted peer hasn't observed the key yet". Neither `on_message` nor `Key` exists; the harness observes by pull (`Peer::drain`) and identity is the `Version`. The executor's own doc (executor.rs:95-100) states the rule correctly. AGENTS.md's hard rule: nothing in the codebase refers to code that no longer exists. The paragraph also opens in the first person plural, which no other module doc in the partition does.

Evidence:

         9	//! We deliberately do *not* compare against an unrestricted run of
        10	//! the same schedule. Doing so would assume order-independence of
        11	//! redactions, but a redact event can only happen at peer `P` once
        12	//! `P` has already received the targeted message via an `on_message`
        13	//! callback —
        14	//! which is a function of the gossip schedule. A partitioned schedule
        15	//! may legitimately suppress some redacts (because the targeted peer
        16	//! hasn't observed the key yet), so the two schedules can converge

Resolution: Rewrite in terms of what is: "a redact event can only fire at peer `P` once the targeted message appears in `P`'s observation log (`Peer::drain`), which is a function of the gossip schedule. A partitioned schedule may legitimately suppress some redacts (the redacting peer has not yet observed the version), so ..." and drop "We". Fix the same `on_message` ghost at tests/shadow_validity.rs:16 in the same pass. Acceptance: `grep -rn on_message tests/ src/` returns nothing; no partition file uses "key" for a message identity.

### tests-lifecycle-20: Peer indices drawn as modulo'd seeds, while the suite documents two competing index idioms
- Where: tests/partition.rs:50-56 (related: tests/redaction.rs:31-33, tests/redaction.rs:48, tests/multi_peer.rs:130-138, tests/party_conservation.rs:124-129)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (all four sites read; multi_peer.rs:130-132 states the `prop_flat_map` rationale; party_conservation.rs:126-129 states the modulo rationale)
- Seen by: api-economics; refutation: reframed (party_conservation.rs documents the modulo form deliberately for a growing fleet, so this is an inconsistency between two documented idioms rather than a lapse against one); history: no-rationale-found (the commit introducing these draws applied `prop_flat_map` to one test and named the reason, leaving the others unchanged)
- Owner-gated: no

`split_at` and `partition_event_count` are `any::<usize>()` reduced modulo the fleet and schedule sizes, as is `redactor_idx` in redaction.rs. multi_peer.rs:130-132 rejects this shape for a fixed fleet ("so the shrinker sees them as first-class inputs rather than modulo'd seeds"), while party_conservation.rs:124-129 adopts it for a fleet that grows mid-schedule ("so every generated schedule is valid at every fleet size"). partition.rs and redaction.rs have fixed fleets, so the first rationale applies and the second does not; a shrunk counterexample here reports a large seed rather than the index it denotes.

Evidence:

        50	        split_seed in any::<usize>(),
        51	        partition_event_seed in any::<usize>(),
        52	        windows in arb_window_assignment(),
        53	    ) {
        54	        let n = schedule.n_peers;
        55	        let split_at = (split_seed % (n - 1)) + 1;
        56	        let partition_event_count = partition_event_seed % (schedule.events.len() + 1);

Resolution: Draw `(schedule, split_at, partition_event_count)` via `prop_flat_map` on the schedule as multi_peer.rs does; in redaction.rs draw `(n_peers, redactor)` with `(2usize..=6).prop_flat_map(|n| (Just(n), 0..n))`. State once (in `common::schedule`'s doc or window.rs's) that modulo resolution is for growing fleets and flat_map for fixed ones. Acceptance: no `% n` index derivation from an `any::<usize>()` over a fixed fleet in the partition's proptest inputs; the rule for choosing between the idioms is written down once.

### tests-lifecycle-21: The redaction no-op tests assert quantities a redaction cannot change and leave the documented no-op contract unpinned; one testdoc says the docs are silent where they speak
- Where: tests/redaction.rs:94-133 (related: tests/common/peer.rs:63-75, tests/common/peer.rs:89-94, src/rumors.rs:176-177, src/batch.rs:79, src/tree.rs:406-408, src/batch.rs:132-143, tests/single_peer.rs:370-384)
- Class / severity / confidence: test-quality / medium / medium
- Provenance: verified for the vacuity (peer.rs:65-75 `drain` records only live leaves above the checkpoint and `redact_one` drains, so `observations.len()` cannot change across a redaction; in `redact_twice_is_idempotent` the only message is already redacted before `readout_before` is taken, and in `redact_unknown_version_is_noop` alice is a fork of an empty seed, so both readout comparisons are over empty maps) and for the doc (rumors.rs:176 "Redacting a version not currently held is a no-op", blame ce27df86e 2026-08-20; batch.rs:79; redaction.rs:117-118 blame 80a3155f41 2026-05-18); assessed for whether the tree's own tests hold the changed-flag contract (not checked)
- Seen by: blind-spots, structure-prose, api-economics; refutation: confirmed, severity lowered to low with the reasoning that the tests are weak rather than wrong; history: deliberate-but-expired for the doc (silent when written, documented 2026-08-20), and the observation-count assert was vacuous from birth
- Owner-gated: no

Both tests can fail only on a panic. The contract these tests exist to pin (an unheld or already-redacted version is a no-op: the causal ceiling does not move, `Tree::act` returns `false`, `Batch::commit`'s `send_if_modified` wakes no observer) is not examined: neither `latest()` nor a `changes()` observer is read. An implementation that ticked the party on an ineffectual forget would pass both tests unchanged. The doc at 116-118 says "the public docs are silent on this corner", which `Rumors::redact` and `Batch::redact` contradict; a test that guards a documented clause should say so, since that is what makes it non-negotiable at review. I hold this at medium against the refutation's low because the doctrine is explicit: a criterion the bad implementation also passes is decoration, and no public-tier test pins the no-tick promise.

Evidence:

       103	        let readout_before = readout_multiset(&peer.local.snapshot());
       104	        let obs_before = peer.observations.len();
       105	
       106	        peer.redact_one(&version);
       107	
       108	        prop_assert_eq!(readout_multiset(&peer.local.snapshot()), readout_before);
       109	        prop_assert_eq!(peer.observations.len(), obs_before);

       116	    /// Pins down the currently implemented behavior so
       117	    /// future regressions surface; the public docs are silent on
       118	    /// this corner.

Resolution: In both tests, give the peer live content that survives the redaction so the readout comparison is not over an empty map; capture `peer.local.snapshot().latest().clone()` before the redaction and assert it is unchanged after; subscribe a `changes()` observer (as single_peer.rs:370-384 does, draining the immediate first tick) and assert `now_or_never()` yields `None` after the redaction. Rewrite the doc at 116-118 to cite the no-op clause `Rumors::redact` states; consider folding the two tests into one property over "a version not currently held (never held, or already redacted)". Acceptance: both tests fail if `Tree::act` (or `Batch::commit`) is mutated to advance the ceiling or report `true` on an ineffectual forget; the testdoc no longer claims the docs are silent.
Construction: mutate `Batch::commit` at batch.rs:143 to `inner.tree.act(party, actions); true` (unconditional wake) or make the ceiling join run for every action: redaction.rs stays green today; the proposed `latest()`/`changes()` assertions fail.

### tests-lifecycle-22: retire.rs prose describes a domination precondition and declines the `Retire` contract does not have
- Where: tests/retire.rs:11-12 (related: tests/retire.rs:114-115, tests/retire.rs:143-145, tests/retire.rs:155, tests/retire.rs:159-161, tests/retire_snapshot.rs:73-74, src/peer/gossip.rs:101-103, src/peer/gossip.rs:453, src/peer.rs:285-287)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -n dominat src/peer/gossip.rs` hits only two bookmark lines, 519 and 650; `Retire::Declined` is produced solely by `(Intent::Remain, Ok(_))` at gossip.rs:453 and documented as "The peer was itself retiring" at 101-102; peer.rs:285-287 states reconcile-then-absorb; fedb3ecb2 (2026-06-09) is "Retire now reconciles before relinquishing the party")
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (143-145 were true when written on 2026-06-08; fedb3ecb2 removed the precondition the next day; the header's "remain" and 159-161's "is not declined" were written after the removal as contrast prose against the old design)
- Owner-gated: no

Five passages describe retirement as gated on the absorber causally dominating the retiree, with divergence "not declined": the header's "Declines remain only" (negative space against a design that declined more), "equal versions, so it reflexively dominates" (114-115), "the `<=` domination precondition" (143-145), "equal versions dominate reflexively, so retire commits" (155), "A retiree whose peer does *not* dominate it ... is not declined" (159-161), and retire_snapshot.rs:73-74's "the absorber dominates reflexively". The public contract has no such precondition: a retire session reconciles exactly as gossip would and then the peer absorbs the identity; `Declined` means only that the counterparty was itself retiring. Prose speaks in the present tense: a testdoc states today's invariant, not what the test would have caught under a design the tree no longer has. (Line 6's "comes to causally dominate the retiree before the party changes hands" is a fine mechanism description and needs no change.)

Evidence:

        11	//! version are untouched). Declines remain only for a counterparty that is
        12	//! itself retiring; a bootstrapping counterparty *absorbs* the retiree —

       143	/// Equal versions satisfy the `<=` domination precondition reflexively: a
       144	/// fresh, empty bootstrap fork can retire into the peer it forked from with
       145	/// no prior gossip.

Resolution: Restate positively: the header says a retire reconciles first and then absorbs, and declines only a retiring counterparty (drop "remain"); `empty_equal_version_retire_succeeds` becomes "an empty fork retiring into its parent is the minimal session: no content moves, the party is absorbed"; `divergent_retiree_reconciles_then_retires` drops "is not declined" and "dominate"; retire_snapshot.rs:73-74 drops "dominates reflexively". Acceptance: `grep -n 'dominat\|not declined\|remain only' tests/retire.rs tests/retire_snapshot.rs` returns only mechanism descriptions of the in-session reconciliation, none naming a precondition or a decline the enum does not have.

### tests-lifecycle-23: The header misplaces where party accounting lives, and the retire-into-bootstrapper point test omits the party check integration tests can make
- Where: tests/retire.rs:16-20 (related: tests/retire.rs:262-294, tests/party_conservation.rs:12-14, tests/party_conservation.rs:60-64, tests/common/sim.rs:1022-1057, tests/membership.rs:72-74, src/rumors.rs:418-424, Cargo.toml:145)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`dangerously_alias_party` is `pub` under `cfg(any(test, feature = "test-internals"))` at rumors.rs:420-422; Cargo.toml:145 enables `test-internals` for the crate's own tests; party_conservation.rs:60-64 and sim.rs:1022-1029 call it; membership.rs:74 calls `assert_party_invariants`; retire.rs:262-294 asserts network, content, and one origination, with no party assertion)
- Seen by: blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired (the header was true for about an hour on 2026-06-10: 1d3a3df4d1 landed at 22:42 and 9eadfc680 exposed the alias at 23:39; party_conservation.rs arrived a month later)
- Owner-gated: no

The header says the party-accounting side "lives in the crate-level tests, which can read the party." Integration tests read the party too, and the gate-permanent accounting suite is `tests/party_conservation.rs`; membership.rs in this very partition asserts seed reconstitution through `assert_party_invariants`. Meanwhile `retire_into_bootstrapper_hands_off_the_identity` checks network, content, and one origination but not that seed ⊔ successor reconstitutes `Party::seed()` and that the two are disjoint, even though the identity hand-off to a newborn is a distinct wire path (the bootstrap side receives the whole party as the trailing frame) and the sharp check is one call away. No generative engine reaches this cross: the membership executor absorbs through plain gossip and party_conservation's `Op::Retire` targets a live fleet member.

Evidence:

        16	//! (No test covers a retire refused by outstanding snapshots, because
        17	//! none can exist: the `Peer`/`Rumors` XOR makes "retire while
        18	//! observers share the party" unrepresentable at compile time. The
        19	//! party-accounting side — every retire reconstituting the seed's whole
        20	//! id-space — lives in the crate-level tests, which can read the party.)

Resolution: Rewrite the sentence to name `tests/party_conservation.rs` and the membership suite's `assert_party_invariants`. In `retire_into_bootstrapper_hands_off_the_identity`, after the hand-off call `crate::common::sim::assert_party_invariants(&[seed.clone(), successor.clone()], 0)`. Optionally add an `into_newcomer` arm to party_conservation's `Op::Retire`. Acceptance: the point test fails if the successor receives a fork rather than the retiree's whole region; the header names the actual accounting suites.
Construction: the assertion `assert_party_invariants(&[seed, successor], 0)` passes today by the retire contract; mutating gossip.rs:702 `inner.party.take()` to `inner.party.as_mut().map(Party::fork)` (donating a fork instead of the whole party on retire) leaves content and network intact and fails only this assertion.

### tests-lifecycle-24: `async_known` is a relic: its qualifier contrasts with a removed surface, `send_all` expresses it, and its doc is false for five call sites
- Where: tests/retire.rs:37-42 (related: tests/retire.rs:112, tests/retire.rs:123, tests/retire.rs:201, tests/retire.rs:226, tests/retire.rs:304, tests/retire.rs:329, tests/retire.rs:193, tests/bootstrap.rs:6, tests/common/wire.rs:1, tests/async_wire.rs:1, tests/pairwise.rs:3-4)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 83edcd944^:tests/retire.rs` has `fn sync_known(peer: rumors::sync::Rumors<u64>, ..)` at line 47 beside `async_known`; `git log -S'pub mod sync' -- src/lib.rs` shows the module removed in 83edcd944 on 2026-07-16; `grep -n 'async_known(' tests/retire.rs` lists twelve call sites, five of which pass `seed` itself; retire.rs:193 already uses `send_all`; 212c6914 (HEAD's parent) added `send_all`)
- Seen by: structure-prose, api-economics; refutation: confirmed (adds the doc inaccuracy for the seed call sites); history: deliberate-but-expired (twice: the sync twin left on 2026-07-16, and `send_all` on 2026-09-01 removed the reason to route fixture inserts through `LocalAction`)
- Owner-gated: no

The helper's `async_` prefix distinguished it from a `sync_known` twin over `rumors::sync::Rumors`, a surface that no longer exists; the `known` half names a type renamed on 2026-06-11. Its body is `peer.send_all(vals.iter().copied()).unwrap(); peer`, which line 193 of the same file already writes. Its doc says it inserts into "a genuine bootstrap fork", but five of twelve call sites pass the seed itself. The section header at line 112 ("async behavioral tests") carries the same dead qualifier, as do common/wire.rs:1 and async_wire.rs:1 ("the *asynchronous* gossip path") and bootstrap.rs:6 ("Mirrors `async_wire.rs`'s setup"). pairwise.rs:3-4 ("there is no in-process `join`") reads as the same negative space, more softly. Names and prose speak in the present tense; a qualifier that contrasts with removed code is a ghost reference in disguise.

Evidence:

        37	/// Build an async `Rumors<u64>` by inserting `vals` into a disjoint originator
        38	/// (a genuine bootstrap fork: its own party region, ready to originate).
        39	fn async_known(peer: Rumors<u64>, vals: &[u64]) -> Rumors<u64> {
        40	    let actions: Vec<LocalAction<u64>> = vals.iter().map(|&v| LocalAction::Insert(v)).collect();
        41	    build_local(peer, &actions)
        42	}

       112	// ---- async behavioral tests ---------------------------------------------

Resolution: Delete the helper; at each call site write `let a = bootstrap_fork(&seed); a.send_all([1, 2]).unwrap();` (or a two-line local `fn with(peer, vals)` if the expression form reads worse); drop the unused `LocalAction` import; rename the section header "behavioral tests"; drop "asynchronous" from wire.rs:1 and async_wire.rs:1 and the "Mirrors" clause from bootstrap.rs:6. Acceptance: `grep -rn 'async_known\|\*asynchronous\*\|async behavioral' tests/` returns nothing; retire.rs builds fixtures through `send_all`.

### tests-lifecycle-25: retire.rs keeps a point test its counting sibling strictly subsumes, an overlapping divergent pair, and a byte-identical oracle block
- Where: tests/retire.rs:114-141 (related: tests/retire.rs:322-348, tests/retire.rs:159-182, tests/retire.rs:296-320, tests/retire.rs:382-390, tests/retire.rs:424-432)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`diff` of retire.rs:121-127 against 327-333 is identical, and `gossip_absorbs_retiree_without_observations` adds `novel == 0` to the same three assertions; `diff` of 382-390 against 424-432 is identical; the divergent pair differ in fixture ([1]/[2] versus [1,2]/[3]) and in assertion (exact live list versus count))
- Seen by: structure-prose, api-economics; refutation: confirmed ("overlap", not subsumption, for the divergent pair); history: deliberate-but-expired (the `gossip_*` twins were distinct at birth because they counted the absorber's `on_message` callbacks, which the `retire_into_*` twins did not exercise; the callback removal and the shared-state port re-spelled that count as a drain and collapsed the distinction)
- Owner-gated: no

`retire_into_converged_peer_succeeds` asserts a strict subset of `gossip_absorbs_retiree_without_observations` over an identical setup. `divergent_retiree_reconciles_then_retires` and `gossip_learns_content_from_divergent_retiree` are two point instances of `unsynchronized_retire_matches_plain_gossip`, differing in fixture and in whether the exact live list or its count is asserted. The "Oracle" blocks of the two wire-equivalence properties are byte-identical. Duplicated oracle construction is where two copies drift (one gets a fix, the other keeps the bug); a subsumed point test adds run and reading time without coverage.

Evidence:

       120	fn retire_into_converged_peer_succeeds() {
       121	    let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
       122	    let a = async_known(bootstrap_fork(&seed), &[1, 2]);
       123	    let b = async_known(seed, &[3, 4]);
       124	
       125	    wire_gossip(&a, &b);
       126	    let pre = b.snapshot();
       127	    let (hash, version) = (pre.hash(), pre.latest().clone());

Resolution: Delete `retire_into_converged_peer_succeeds`; keep one divergent point test as the legible worked example beside the property, folding the exact-content assertion into it; extract `fn plain_gossip_oracle(a_actions, b_actions) -> Fingerprint` and call it from both properties. Acceptance: retire.rs has no two tests with identical setup lines, one plain-gossip oracle construction, and every assertion the deleted tests made present in a survivor.

### tests-lifecycle-26: retire_redaction.rs is a one-test binary restating a retire.rs claim with a hand-rolled session and an application-scenario framing
- Where: tests/retire_redaction.rs:1-8 (related: tests/retire_redaction.rs:39-46, tests/retire.rs:52-65, tests/retire.rs:184-217)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (file read in full, 54 lines, one test; its session at 39-46 respells `retire_into_gossip` at retire.rs:52-65 including the drain assert; retire.rs:184-217 already pins a retiree's redaction propagating through the retire session)
- Seen by: structure-prose, api-economics; refutation: confirmed, severity lowered to low; history: deliberate-but-expired (born in 374adc960 as a regression pin beside the rumormill soak harness and framed in rumormill's chatroom vocabulary; rumormill left the workspace in 397fcb081 on 2026-07-31; the retire.rs sibling already existed the day the file was created)
- Owner-gated: no

The single test proves that a redaction the retiree performed after its last gossip reaches the absorber through the retire session. retire.rs already tests this claim with a different construction (message inserted before the fork versus originated by the retiree and learned via gossip), and neither doc names that difference as the reason for two tests. The module doc frames the test as an application scenario ("chatroom goodbye path", "presence entry") rather than by the mechanism it names in passing ("deletion honoring rides version bounds"). Each `tests/*.rs` file is a separate binary linking `common`, so this is a full link unit for one test, and a reader of retire.rs will not know the second construction exists.

Evidence:

         1	//! A retiree's last-moment redactions must survive into the absorber.
         2	//!
         3	//! This is the chatroom goodbye path: a departing peer redacts its own
         4	//! presence entry, then retires its party into a live peer. The retire
         5	//! session's built-in reconciliation must carry the *absence* (deletion
         6	//! honoring rides version bounds), not just the retiree's unsent content —
         7	//! otherwise every clean departure leaves a ghost entry behind that only
         8	//! application-level staleness sweeps can clear.

Resolution: Move the test into retire.rs, drive it through `retire_into_gossip` (or the shared driver of tests-lifecycle-3), and either merge it with `retiree_redaction_propagates_through_retire` as two arrangements of one test or state in its doc what distinguishes the gossip-learned construction (the redacted version lives above the absorber's frontier, so the absence must be inferred from the retiree's ceiling rather than from shared pre-fork state). Delete tests/retire_redaction.rs and rewrite the framing as mechanism. Acceptance: tests/retire_redaction.rs is gone; retire.rs carries the gossip-learned case with a doc naming what distinguishes it; `just test retire` passes.

### tests-lifecycle-27: reuse.rs takes a Tokio runtime and a wall-clock deadline where the harness's `block_on` witnesses a wedge deterministically
- Where: tests/reuse.rs:23-26 (related: tests/reuse.rs:61, tests/reuse.rs:97, tests/reuse.rs:136, tests/reuse.rs:193, tests/reuse.rs:238, tests/reuse.rs:70-74, tests/common/wire.rs:34-48, src/testing.rs:366-394, tests/lifecycle.rs:99, .config/nextest.toml:1-5)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (all five tests are `#[tokio::test(flavor = "current_thread")]` and the only Tokio facility used is `timeout`, at lines 70, 115, 145, 173, 199, 211, 242; `tokio::io::AsyncWriteExt::write_all` on the memory duplex needs no runtime; lifecycle.rs:99 already drives `wrap_link`ed links under `run_to_quiescence`; `run_to_quiescence` caps at `MAX_POLLS = 1_000_000` at testing.rs:376)
- Seen by: structure-prose, api-economics; refutation: confirmed with the `MAX_POLLS` caveat; history: deliberate-but-expired in part (the idiom predates `run_to_quiescence` by five weeks; ba3eacb1ad kept the repeated block "deliberately alone", but its stated reason concerns extraction, not the runtime)
- Owner-gated: no

`common::wire::block_on` returns `Quiescence::Stalled` the moment a closed-world future goes `Pending` without arranging a wake, which is exactly the wedge `DEADLINE` guards against, and the harness's own guidance reserves Tokio for tests that need "task spawning, timers, or networking" (wire.rs:44-48). A wall-clock threshold over a deterministic quantity relocates flakiness to the threshold: a stall here costs ten seconds and a timeout panic instead of an immediate `Stalled` at the failing poll. The recorded ruling in ba3eacb1ad declined to extract the repeated block; it did not rule on the runtime.

Evidence:

        23	/// Generous wall-clock bound: these sessions are in-memory and finish in
        24	/// microseconds, so hitting the deadline means lost bytes wedged a session,
        25	/// not a slow machine.
        26	const DEADLINE: Duration = Duration::from_secs(10);

Resolution: Make the tests plain `#[test]`s that call `block_on(async { tokio::join!(..) })` per round (per round, not once around the body: the 259-session epoch-wrap test should stay well inside `MAX_POLLS`); delete `DEADLINE`, the `timeout` calls, and the `Duration` / `tokio::time` imports. If the runtime flavor is deliberate (coverage under a real executor, which .config/nextest.toml's comment names as a stall class), say so in the module doc instead. Acceptance: reuse.rs has no `tokio::test`, `timeout`, or `DEADLINE`, or its module doc states why it runs under a real executor; a deliberately planted wedge fails with `Stalled` rather than after 10 s.

### tests-lifecycle-28: The epoch-wrap test infers the wrap from hand-computed constants and never reads the public `SessionState::epoch()`
- Where: tests/reuse.rs:184-194 (related: tests/reuse.rs:36-47, tests/reuse.rs:198-226, tests/reuse.rs:153-168, tests/reuse.rs:252-258, src/link.rs:346-352, src/link.rs:363-367, src/link.rs:385-393, src/link.rs:470-478, src/link.rs:499)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (reuse.rs:194-226 read: nothing observes the epoch; link.rs:352 `epoch: u8`, :391 `wrapping_add(1)`, :365-367 `pub fn epoch(&self) -> u8`, :499 `pub session: SessionState` on `LinkParts`; reuse.rs:252-258 already uses `into_parts`/`into_link`)
- Seen by: structure-prose, blind-spots; refutation: confirmed; history: deliberate-but-expired (when the test was written, `SessionState`'s fields were not publicly readable; ee67e4cbb exposed `epoch()` six days later)
- Owner-gated: no

The test runs 253 no-op sessions then 6 divergent rounds and asserts convergence; the doc states the rounds run "at epochs 253 through 255 and the wrapped 0 through 2", but nothing observes an epoch. `PRE_WRAP_SESSIONS = 253` is a hand-derived literal tied to the `u8` width. If the epoch became `u16`, or `begin` stopped counting stream-less sessions, the wrap would never occur and the test would keep passing as an ordinary reuse test. `empty_sessions_advance_epochs_in_lockstep` (153-168) asserts its own premise through the wrapped link's counters; this test does not. Every bound needs proof its signal is alive; state the structure ("two before the wrap") and let the compiler compute the tally.

Evidence:

       189	/// Strategy: converged no-op sessions burn epochs cheaply (they
       190	/// open no data streams but still count — the lockstep the test above
       191	/// pins), then divergent rounds bracket the wrap itself, running at
       192	/// epochs 253 through 255 and the wrapped 0 through 2.

        42	const PRE_WRAP_SESSIONS: usize = 253;

Resolution: `const PRE_WRAP_SESSIONS: usize = u8::MAX as usize - 2;` with the doc saying "two epochs before the wrap". After the pre-wrap loop, on each end: `let parts = link.into_parts(); assert_eq!(parts.session.epoch(), PRE_WRAP_SESSIONS as u8); let mut link = parts.into_link();`. After the wrap rounds, assert both ends read `((PRE_WRAP_SESSIONS + WRAP_ROUNDS as usize) % 256) as u8`, which witnesses the lockstep directly. Acceptance: changing `PRE_WRAP_SESSIONS` so no wrap occurs, or widening the epoch type, fails the test at the epoch assertion; no literal 253 in reuse.rs.

### tests-lifecycle-29: sanity.rs's panic-freedom property is strictly subsumed by multi_peer, and its doc's premise is false
- Where: tests/sanity.rs:19-29 (related: tests/sanity.rs:15-16, tests/multi_peer.rs:21-22, tests/multi_peer.rs:40-44, tests/sanity.rs:31-69, tests/sanity.rs:72-93, .config/nextest.toml)
- Class / severity / confidence: performance / low / high
- Provenance: verified (sanity.rs:15-16 and multi_peer.rs:21-22 define identical `N_PEERS` (2..=8) and `MAX_EVENTS` (50); sanity.rs:25-28 draws `arb_schedule(any::<u64>(), ..)` with `arb_window_assignment()` and calls `execute_and_quiesce`, exactly what every multi_peer property does before asserting more; tests run as independent processes under nextest and as independent functions under libtest)
- Seen by: api-economics; refutation: confirmed; history: no-rationale-found (the premise was never literally true even in the original single-binary suite; the split into per-file binaries and nextest made it less so)
- Owner-gated: no

`arbitrary_schedules_dont_panic` adds 256 executor passes and no sampled space: any panic it would catch already fails every multi_peer property on the same generator. Its doc says "if this fails, the others cannot run", but the others run and fail with the same panic. A test whose every failure is already a failure of a stronger committed test is pure cost, and an inaccurate testdoc is a bug in the test. With it gone, `forked_gossip_matches_direct_gossip` belongs with the pairwise laws and `quiesce_handles_zero_or_one_peer` is a harness self-test, so the binary dissolves.

Evidence:

        20	    /// Arbitrary schedules complete without panicking and produce a
        21	    /// finite converged state. The safety net for every other
        22	    /// invariant in the suite — if this fails, the others cannot run.
        23	    #[test]
        24	    fn arbitrary_schedules_dont_panic(
        25	        schedule in arb_schedule(any::<u64>(), N_PEERS, MAX_EVENTS),
        26	        windows in arb_window_assignment(),
        27	    ) {
        28	        let _ = execute_and_quiesce(&schedule, &windows);
        29	    }

Resolution: Delete the test. Move `forked_gossip_matches_direct_gossip` into pairwise.rs and `quiesce_handles_zero_or_one_peer` beside the harness's other self-checks (shadow_validity.rs, or wherever the harness self-tests are collected), then delete sanity.rs. Acceptance: tests/sanity.rs no longer exists or contains no test another binary's property implies; both surviving tests still run.

### tests-lifecycle-30: single_peer.rs's module doc covers half the file
- Where: tests/single_peer.rs:1-6 (related: tests/single_peer.rs:29-140, tests/single_peer.rs:171-462)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read: the doc names fan-out, version distinctness, and monotonicity, which is the proptest block at 29-140; lines 171-462 pin commit-iff-Ok, cancel on panic, on `?`-propagated depth error, and on user `Err`, `send_all` all-or-nothing, `redact_all` skip-and-one-tick, the locally handled admitted prefix, and inner-before-outer nesting)
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (the doc's last edit coincides with the first batch-lifecycle additions and predates the second batch)
- Owner-gated: no

A module doc is the file's map; when it omits half the contents, the next batch-lifecycle test lands in the wrong file or gets duplicated elsewhere.

Evidence:

         1	//! Single-peer correctness for a lone rumor set, with no gossip.
         2	//!
         3	//! Exercises the surface area of [`Batch`](rumors::Batch) commits:
         4	//! live-leaf fan-out, distinctness of the [`Version`](rumors::Version)s
         5	//! created within a batch, and strict monotonicity of the local party's
         6	//! component of each created version.

Resolution: Extend the doc with one sentence per group: the commit lifecycle (commit on `Ok`, cancel on `Err` or panic), the bulk variants' all-or-nothing and skip semantics, and nesting. Acceptance: every test in the file is covered by a sentence in the module doc.

### tests-lifecycle-31: Version recovery after `send` is reimplemented in six shapes; the harness already has `created_version`, and the corpus is evidence for the API decision
- Where: tests/single_peer.rs:20-27 (related: tests/single_peer.rs:216-221, tests/single_peer.rs:357-364, tests/retire.rs:194-198, tests/retire_redaction.rs:25-30, tests/pairwise.rs:193-207, tests/common/action.rs:47-64, tests/common/peer.rs:77-87, src/rumors.rs:143-149, src/rumors.rs:196-207)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (`grep -ln created_version tests/*.rs` lists causal.rs, changes.rs, gossip_when.rs, listen.rs, four binaries and none in this partition; `grep -rnE 'find_map\(\|\(v, m\)\|' tests/` lists retire.rs:197 and three sites outside the partition; the other shapes read at the cited lines; rumors.rs:143-149 and 196-207 state the no-return decision with its two reasons)
- Seen by: api-economics, structure-prose; refutation: confirmed (the "five binaries" count corrected to four); history: deliberate-and-holds for the API decision (its rationale is stated inline in the code), no-rationale-found for the harness duplication
- Owner-gated: yes for the API half (whether `send` should return a `Version`, or the docs should name the recommended recovery idiom); no for the harness half

`Rumors::send` deliberately returns no `Version`. The tests, the crate's heaviest user, then recover it after every send in six shapes: `batch_send` here (pre-frontier snapshot then `range(causally::since(&pre))`), `Peer::insert_one` (drain-based), `created_version` (the harness helper built for exactly this), `find_map` by payload (retire.rs, single_peer.rs:357-364), `.iter().map(|(v, _)| v.clone()).next()` (retire_redaction.rs, single_peer.rs:216-221), and `range(since).next()` inline (pairwise.rs). For the harness, one idiom used everywhere is the legibility rule. For the API, the inline rationale rests on the observe-then-redact shape being the natural one; the corpus is evidence about how often callers instead need the version at the write site, worth weighing knowingly rather than leaving implicit in six workarounds.

Evidence:

        20	fn batch_send(peer: &Rumors<u64>, values: &[u64]) -> Vec<Version> {
        21	    let pre = peer.snapshot().latest().clone();
        22	    peer.send_all(values.iter().copied()).unwrap();
        23	    peer.snapshot()
        24	        .range(causally::since(&pre))
        25	        .map(|(v, _)| v.clone())
        26	        .collect()
        27	}

Resolution: Harness: use `common::action::created_version` (and a sibling `created_versions` for batches) at every single-send site, and add `version_of(&Snapshot<T>, &T) -> Version` to `common::oracle` for the by-payload lookups. API (owner): decide whether `send` should return the version after all, or whether `Rumors::send`'s docs should name the recommended recovery idiom (`snapshot().range(causally::since(&pre))`) so users do not each re-derive it. Acceptance: no partition file contains an inline `range(causally::since(&pre)).next()` or a by-payload `find_map` for version recovery; the API decision is recorded either way.

### tests-lifecycle-32: A testdoc cites a batch-docs promise of strictly increasing per-action versions that the public docs do not make
- Where: tests/single_peer.rs:66-69 (related: src/batch.rs:10-26, src/tree.rs:370-382, tests/single_peer.rs:84-87)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i 'increasing\|ascending\|monoton\|totally ordered\|strictly' src/batch.rs src/peer.rs src/lib.rs src/rumors.rs` returns nothing relevant; src/batch.rs read in full, it mentions versions only for redaction; the only statement is the private `Tree::act` doc at tree.rs:373-376; `git show 8dc0596ed -- src/batch.rs` deletes the sentence "increasing versions (content-identical messages get distinct keys)" that the testdoc cited)
- Seen by: structure-prose, blind-spots; refutation: confirmed; history: deliberate-but-expired (the promise existed in `Batch`'s public docs the day the testdoc was written, 2026-06-10, and 8dc0596ed removed it the next day)
- Owner-gated: no (the reword needs no API decision; restoring the clause would, and is listed as an open question)

The property is sound (a lone party's versions form a chain), but its doc attributes the guarantee to "the batch docs", which no longer state it; a reader looks for the promise, fails to find it, and cannot tell whether the test or the docs is wrong. The test also cannot check action order: `batch_send` recovers versions in tree-iteration order and the test sorts them (84-87).

Evidence:

        66	    /// Every `Version` created by a lone peer is totally ordered against
        67	    /// every other — both within a single batch (the batch docs promise
        68	    /// strictly increasing versions per action) and across successive
        69	    /// batches.

Resolution: Reword the parenthetical to state the invariant in its own terms ("every action ticks the one party, so a lone peer's versions are a chain") or cite `Tree::act` as the maintainer contract; alternatively, if Finch wants users to rely on per-action ordering within a batch, restore the clause to `Batch`'s docs and keep the citation. Acceptance: the doc names a contract that exists where it says it exists.

### tests-lifecycle-33: A hand-rolled Fisher-Yates over an inline LCG where `rand` and proptest already provide a shuffle
- Where: tests/single_peer.rs:109-124 (related: Cargo.toml:165, tests/bootstrap_snapshot.rs:25-26)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (Cargo.toml:165 `rand = { workspace = true, features = ["small_rng"] }` under `[dev-dependencies]`; the snapshot suites use `SmallRng`)
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (no `rand` in the manifest when the LCG was written on 2026-05-18; `small_rng` arrived the next day; the "no extra dependency" comment was added two months later when the premise was already false)
- Owner-gated: no

The shuffle carries two unnamed 64-bit constants and a comment justifying it as avoiding a dependency, but `rand` with `small_rng` is already a dev-dependency, and proptest's `prop_shuffle` would make the permutation a first-class strategy input the shrinker can simplify. Prefer a dependency over hand-rolling; named constants over magic numbers; the stated premise is false for this crate.

Evidence:

       111	            // Fisher-Yates over an inline 64-bit LCG: deterministic
       112	            // from `seed`, no extra dependency; any step function whose
       113	            // high bits reduce to a uniform-enough draw over `0..=i`
       114	            // would do.
       115	            let mut state = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
       116	            for i in (1..v.len()).rev() {
       117	                state = state
       118	                    .wrapping_mul(6364136223846793005)
       119	                    .wrapping_add(1442695040888963407);

Resolution: Take `(values, shuffled) in vec(any::<u64>(), 0..=16).prop_flat_map(|v| (Just(v.clone()), Just(v).prop_shuffle()))`, or `values.shuffle(&mut SmallRng::seed_from_u64(seed))`; delete the LCG. Acceptance: no literal LCG multiplier or increment in single_peer.rs; the test passes under both committed proptest seeds.

## Positives

- Session-boundary hygiene is uniform: every successful in-memory session in the partition ends in `assert_control_drained` (the two exceptions are tests-lifecycle-7), and reuse.rs:228-267 commits the negative control that plants one byte and requires the assert to fire, so the gate is itself gated against rot.
- The `Joined`-arm tests in bootstrap.rs (234-411) each name and run their negative control (empty-store precondition, retry through the returned bookmark against a live provider, the fault-free identical join), so the arm reached is attributable to the injected condition.
- lifecycle.rs's `a_lost_epilogue_marker_is_distinguished_and_post_commit` (166-234) measures the clean byte schedule on a byte-identical probe pair and replays one byte short; its error-class and content assertions would fail differently under a misplaced cut, so the test validates its own instrument. The cancellation test (73-94) brackets the cut both ways: each poll asserted pending, then stream counters prove the descent had begun.
- `membership_population_contains_churn` (membership.rs:99-135) is a liveness floor on a generated dimension under the deterministic runner, with a doc stating exactly what would rot without it: the pattern tests-lifecycle-15 asks to extend.
- `empty_sessions_advance_epochs_in_lockstep` (reuse.rs:153-168) asserts its own premise (zero data streams in the converged session) through the wrapped links' counters instead of assuming it, and says why.
- Deterministic floor legs sit beside every swept leg (membership.rs:85-96, multi_peer.rs:168-180), keeping the capacity-one orderings the deadlock argument certifies exercised on every run rather than with generated probability.
- Content checks go through the `readout` / `readout_multiset` lens rather than party state, and retire.rs's wire-equivalence properties (352-443) use the crate's own plain gossip as the differential oracle: an oracle-shaped family stated as a property.
- The schedule executor never swallows an outcome: every `Retire` variant other than `Retired` panics with a message naming why a clean wire forbids it (executor.rs:266-273), and `quiesce_refs` (peer.rs:140-174) stops on identical fingerprints, the fixed point itself, with a bounded loop whose panic names the two things non-convergence could mean.
- single_peer.rs pins the batch lifecycle through the public API at every exit (Ok, user Err, depth Err, panic), including the admission-stops-at-rejection count and the one-tick-per-commit observer check.
- The snapshot suites fix the A/B party convention and its exceptions at the top of the module (bootstrap_snapshot.rs:14-21, retire_snapshot.rs:17-25), exactly what a reader of a hexdump needs.
- No leftovers of the V1 protocol, BLAKE3, or the height and item erasure; no ignored tests, commented-out code, or debug prints anywhere in the partition.

## Open questions for Finch

- reuse.rs runs under `#[tokio::test]` with a 10 s deadline (tests-lifecycle-27). Is real-executor coverage the intent? Recommendation: convert to `block_on` per round; if the executor is the point, say so in the module doc and keep the deadline.
- `Batch` once promised strictly increasing per-action versions and no longer does (tests-lifecycle-32). Restore the clause as a public promise, or reword the testdoc? Recommendation: reword; restore only if users are meant to rely on within-batch ordering, since a batch's versions carry no input-order correspondence at recovery time.
- `Peer::seed_rng` and `warm_caches` are `#[doc(hidden)] pub` with no `cfg` (tests-lifecycle-10). Document or gate? Recommendation: gate both under `test-internals` unless deterministic `Network` seeding is a capability you want application test suites to have; if it is, document the shared-`Network` hazard.
- The dead `# shrinks to n = 1` seed line (tests-lifecycle-1). Recommendation: remove it in a commit naming the deleted property, and extend `seed_liveness.rs` to match `cc` parameter names against live `proptest!` signatures so the class cannot recur.
- multi_peer's one-named-test-per-invariant policy (80a3155f41) versus fusing the two implied readout checks (tests-lifecycle-9). Recommendation: fuse the two implied checks into the canonical-map test with distinct messages, and state the policy in the module doc for the tests that remain separate.
- `Rumors::send` returns no `Version`, and the test corpus recovers it six ways (tests-lifecycle-31). Recommendation: keep the decision and add the recommended recovery idiom to `send`'s docs; revisit only if application code shows the same six shapes.
- Rename the harness `common::peer::Peer` (for example `SimPeer`) to release the forced `rumors::Peer::` qualifications in redaction.rs and sanity.rs (tests-lifecycle-12)? Recommendation: yes; it is a mechanical rename inside tests/.
- Are the pairwise laws deliberately restricted to disjoint-content populations, with the schedule engines the designated home for shared-then-redacted content (tests-lifecycle-17)? Recommendation: yes, and write that division into pairwise.rs's header; extending the laws' population is optional.
- Should retire-into-bootstrap enter a generative alphabet (party_conservation's `Op`, or the membership executor), or is the point test plus the snapshot pin sufficient (tests-lifecycle-23)? Recommendation: add the party assertion to the point test first; a generative arm is a nice-to-have.
- None of the schedule-executor binaries set a `ProptestConfig`, so each property runs the default 256 cases at up to 8 peers and 50 events, while session_overlap (48), disruption (8), and party_conservation (32) budget explicitly. Chosen or inherited? Recommendation: if 256 is fine on the gate's wall clock, leave it and say so in one module doc; otherwise budget the swept legs and keep the floor legs at 256.

## Dropped

- Harness self-tests are scattered across three behavioral suites [18]: refuted; each placement has a stated local charter (sanity.rs names "degenerate inputs", reuse.rs's negative control guards the invariant reuse.rs exists to pin, membership.rs's floor guards its own generator), and no doctrine rule requires centralizing them; taste with a defensible status quo.
- The window sweep never configures the newcomer's side of a bootstrap session [41]: refuted by src/peer/bootstrap.rs:123-128, which documents that the window setting has nothing to bound during a join (an empty replica disputes no subtrees); the sweep's asymmetric-window claim concerns reconciliation sessions, where it holds.
- tests/common/mod.rs's composition map omits `overlap` and `shape` [52]: verified true, but out of this partition (tests/common); route to the harness partition so it is not lost.
- Em-dashes in `//` comments at reuse.rs:144 and 156 [17]: below the bar; the repo holds no such convention (the justfile's comments use em-dashes throughout and tools/ has no dash lint), so this is Claude's own writing discipline, not a finding against the tree. The "silently destroy" tell from the same candidate is folded into tests-lifecycle-4 and the scare quotes into tests-lifecycle-12.
- `durable_bookmark` re-spells the flaky-bookmark construction [19]: below the bar; two sites share the shape (bootstrap.rs:200-204 and bookmark_attach.rs:54-56, the latter outside the partition), the constructor would live in tests/common, and the refutation showed bookmark_causality.rs's per-peer labels would not use it.
- wire_bootstrap duplicates bootstrap_fork_configured [0]: merged into tests-lifecycle-3.
- retire.rs's LINK_BUF cites other suites' headroom [1] and LINK_BUF redefined per binary [45]: merged into tests-lifecycle-2.
- seeded() copied into five binaries [2], [50]: merged into tests-lifecycle-8 (four identical copies; opening_supply's is a distinct fixture).
- retire_redaction.rs restates a retire.rs claim [3], [39]: merged into tests-lifecycle-26 and tests-lifecycle-25.
- partition.rs cites on_message [5], [32], [36]: merged into tests-lifecycle-19, which adds the `key` ghost.
- redaction.rs testdoc says the docs are silent [6], [43] and the no-op tests are vacuous [24]: merged into tests-lifecycle-21.
- batch docs promise [7], [33]: merged into tests-lifecycle-32.
- Lenses re-implemented per suite [10]: fingerprint and canonical map in tests-lifecycle-14; version-by-payload in tests-lifecycle-31.
- retire.rs duplicate oracle block and subsumed point tests [11]: merged into tests-lifecycle-25.
- Twin bootstrap bodies [12] and local pair builders [20]: merged into tests-lifecycle-6.
- reuse.rs tokio and deadline [13], [42]: merged into tests-lifecycle-27.
- sanity.rs brace blocks [14]: merged into tests-lifecycle-13 with `dup` and single_peer.rs's `sync_window_floor`.
- async_known relic [15], [44]: merged into tests-lifecycle-24, which adds the line-112 section header and the seed-call-site doc inaccuracy.
- Import idioms [16], [51]: merged into tests-lifecycle-12 and tests-lifecycle-13.
- Epoch-wrap constants hand-computed [21] and epoch never read [26]: merged into tests-lifecycle-28.
- retire header misplaces party accounting [49] and the bootstrapper cross omits party checks [29]: merged into tests-lifecycle-23.
- "Serve a bootstrap" spelled five times [37]: merged into tests-lifecycle-3.
