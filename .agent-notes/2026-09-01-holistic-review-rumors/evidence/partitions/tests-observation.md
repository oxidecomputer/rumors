# Partition tests-observation: Integration tests: causal and unordered observers, changes, observe hooks, listen, session overlap and stats, party conservation

## Partition summary

This partition is the public-surface behavioral suite for everything an application observes of a replica. Three set observers are driven step by step with `now_or_never`: `UnorderedMessages` (tests/listen.rs), `CausalMessages` (tests/causal.rs), and `Changes` (tests/changes.rs). The wire observation hook is held differentially against the recording-link capture in both directions (tests/observe.rs). The per-session counters are re-checked where an application reads them, with the byte counters compared against an independent transport-level tally (tests/session_stats.rs). The overlapped-session regime has a total deterministic sweep and a generated twin (tests/session_overlap.rs), the schedule generator's shadow simulator has a meta-test against the executor (tests/shadow_validity.rs), the ITC identity algebra is pinned over lifecycle schedules (tests/party_conservation.rs), and one regression pin covers the stale-floor hazard (tests/stale_floor.rs). All 3230 lines read are test code; every file in the partition is a test binary under tests/, and I also read the shared harness they lean on (tests/common/wire.rs, oracle.rs, overlap.rs, schedule/executor.rs, peer.rs, sim.rs) and the production docs the tests pin.

The suite is in good shape. Invariants are stated in English and the bodies check them; families are proptests with committed seeds; the oracles are independent of the code under test (a transport capture for the hook, a transport tally for the byte counters, a BTreeMap oracle for the overlap fleet, `Party::seed()` for the identity fold); every session in the partition runs under `run_to_quiescence`, so a stall fails at the poll instead of hanging; and every in-memory session that owns both link ends finishes with `assert_control_drained`. The negative control in listen.rs and the restart-shaped at-least-once tests in causal.rs are model examples of constructing the counterexample rather than arguing about it.

The dominant issues are residue of mechanical passes that each stopped one step short: fourteen `§6.n` tags from a plan file deleted from the tree; "borrowed faces" and "lent" vocabulary from the dissolved lending API; braces left around single `send_all` calls after the batch-closure rewrite; a first-sentence split that left two-word fragments on their own lines. On structure, the two observer suites carry the same `Step`/`step`/`drain`/`live_map` helpers where the public `try_next` and `oracle::readout` already exist; a retire driver and a 64 KiB link constant are copied across six suites with a rationale the harness itself falsifies (the schedule executor retires at 8 KiB). Three verification gaps deserve scheduling: the gate's `testdoc` regex does not recognize `#[pollster::test]`, so seven tests in changes.rs sit outside doc enforcement (verified by running the tool); the session-stats conservation property never generates a redaction, so its `shed` term is identically zero; and the overlap generator's shadow has no validity meta-test while its executor degrades a mismatch to a skipped event. One contract disagreement is an owner ruling: causal.rs pins a replica-independent delivery order for concurrent messages that the public `CausalMessages` doc deliberately declines to promise.

## Findings

### tests-observation-1: causal.rs's module doc scopes out the unordered face that two of its tests exercise
- Where: tests/causal.rs:1-3 (related: tests/causal.rs:89-99, tests/causal.rs:570-580, tests/causal.rs:620-632, tests/causal.rs:657-667)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read)
- Seen by: structure-prose [11]; refutation: confirmed; history: deliberate-and-holds for the placement (the crash boundary was found on the causal face with the unordered face as control, 6269aab3f; the module doc was never updated)
- Owner-gated: no

The module doc says this file covers `CausalMessages` "on top of everything `UnorderedMessages` already promises (exercised in `tests/listen.rs`)", yet `restart_replays_every_unhandled_message` and `final_pop_checkpoint_still_replays_the_last_message` each run an `UnorderedMessages` half, and `drain_unordered` exists only to serve them. A reader hunting for the unordered face's crash-boundary pin looks in listen.rs and does not find it. A module doc's first sentence stands alone in a listing and tells the reader what lives here; the map is wrong.

Evidence:

    1	//! The [`CausalMessages`] observer: the causal-delivery contract on top of
    2	//! everything [`UnorderedMessages`](rumors::UnorderedMessages) already
    3	//! promises (exercised in `tests/listen.rs`).

    91	fn drain_unordered(obs: &mut rumors::UnorderedMessages<u64>) -> Vec<(Version, u64)> {

Resolution: Keep the two-face tests together (they pin that both faces hold the same checkpoint boundary, which causal.rs:645-649 states) and restate the module doc: "...and the checkpoint-resume boundary both observer faces share, pinned here against both." `drain_unordered` dissolves with tests-observation-2. Acceptance: the module doc names the unordered-face tests present in the file.

### tests-observation-2: Observer step/drain/live_map helpers are duplicated across causal.rs and listen.rs, and reimplement the public `try_next` and `oracle::readout`
- Where: tests/causal.rs:26-99 (related: tests/listen.rs:28-71, tests/common/sim.rs:586-592, src/rumors/unordered.rs:175-182, src/rumors/causal.rs:136-143, tests/common/oracle.rs:80-88)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read both helper blocks and compared line by line; read `try_next` in both observers and `readout` in oracle.rs; `git log -S` dates listen.rs to 040a0d042 on 2026-06-10, `try_next` to 586911cfd on 2026-06-17, `readout` to 8b1ade010 on 2026-05-27)
- Seen by: structure-prose [2], blind-spots [37], api-economics [41]; refutation: confirmed (one caveat: sim.rs's inline drain asserts per item, so a Vec-returning `drain` does not slot in there verbatim); history: no rationale found (causal.rs copied listen.rs's helpers 35 minutes after it landed)
- Owner-gated: no

causal.rs:26-58 and listen.rs:28-60 define the same `Step` enum, `step`, and `drain`, differing only in the observer type; causal.rs:79-87 and listen.rs:62-71 define the same `live_map` (already `readout(&rumors.snapshot())` in tests/common/oracle.rs); causal.rs:89-99 adds a `try_next`-based `drain_unordered`, so one file drains one observer face two ways; tests/common/sim.rs:586-592 inlines a fourth `while let Some(Some(..)) = obs.next().now_or_never()` loop. `step` is the public `try_next` (src/rumors/unordered.rs:175-182) with the `Arc` dereferenced. Since both observers' `Stream::Item` is the owned `(Version, Arc<T>)`, one generic definition serves every site. Duplicated harness code drifts: the two `live_map` docs already differ.

Evidence:

    38	/// Poll the observer exactly once without an executor.
    39	fn step(obs: &mut CausalMessages<u64>) -> Step {
    40	    match obs.next().now_or_never() {
    41	        None => Step::Quiet,
    42	        Some(None) => Step::Ended,
    43	        Some(Some((v, m))) => Step::Item((v, *m)),
    44	    }
    45	}

    (src/rumors/unordered.rs)
    175	    pub fn try_next(&mut self) -> TryNext<T> {
    176	        use futures::{FutureExt, StreamExt};
    177	        match self.next().now_or_never() {
    178	            None => TryNext::Quiet,
    179	            Some(None) => TryNext::Ended,
    180	            Some(Some(message)) => TryNext::Message(message),
    181	        }
    182	    }

Resolution: Add `tests/common/observer.rs` with `enum Step<T>`, `fn step<T: Copy, S: Stream<Item = (Version, Arc<T>)> + Unpin>(obs: &mut S) -> Step<T>` (or return `TryNext<T>` directly and match on it), and `fn drain<...>(obs: &mut S) -> (Vec<(Version, T)>, bool)`; import them in causal.rs and listen.rs; delete both `Step` enums, both `step`/`drain` pairs, `drain_unordered` (its sole caller at causal.rs:623 ignores termination), and both `live_map`s in favor of `readout(&r.snapshot())`. Leave sim.rs's per-item asserting loop, or give the shared `drain` a per-item callback. Acceptance: `grep -rn 'fn step\b\|fn drain\b\|fn live_map\|^enum Step' tests/` matches only under tests/common (bookmark_causality.rs's unrelated simulation `Step` aside).

### tests-observation-3: Causal tests pin a deterministic, replica-independent delivery order that the public `CausalMessages` doc declines to promise
- Where: tests/causal.rs:156-182 (related: tests/causal.rs:122-124, tests/causal.rs:143-153, src/rumors/causal.rs:18-21, src/rumors/causal.rs:54-60, src/rumors.rs:379-381)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (read both texts; `Version::rank` is public in `before`)
- Seen by: blind-spots [24]; refutation: confirmed; history: deliberate-and-holds (8dc0596ed on 2026-06-11 replaced "deterministic linear extension of the causal order" with the "may differ between replicas" sentence while moving engine internals out of user-facing prose; the tests, one day older, pin the internal invariant the private field doc still states)
- Owner-gated: yes: the resolution either amends the public contract or re-labels the tests

`delivery_order_is_replica_independent` asserts two converged replicas replay concurrent messages in the identical sequence, and `converged_backlog_has_no_inversions` asserts strictly increasing `(rank, canonical bytes)` order within a backlog. The public contract (src/rumors/causal.rs:18-21) says concurrent messages come "in arbitrary order, which may differ between gossiping replicas". The private field doc (54-60) says delivery is "causal and deterministic". A reader cannot tell which text is the specification of record: an implementation honoring the public promise with a different tiebreak fails these tests, and a user who could rely on replica-independent replay is not told. The goal stands beside the mechanism; here the test's doc ("the rank order is a property of the set") states a contract the public doc withholds.

Evidence:

    156	/// Two converged replicas deliver the same backlog in the *same* order to
    157	/// fresh observers: the rank order is a property of the set, not of the
    158	/// replica, the insertion order, or the gossip schedule.
    ...
    178	    assert_eq!(
    179	        from_a, from_b,
    180	        "identical sets replay identically, replica notwithstanding"
    181	    );

    (src/rumors/causal.rs)
    18	/// For any two yielded messages with versions `v` and `w`, if `v < w` then the
    19	/// `v` message is yielded first. Concurrent messages are delivered in arbitrary
    20	/// order, which may differ between [`gossip`](crate::Rumors::gossip)ing
    21	/// replicas of the same [`Rumors`](crate::Rumors).

Resolution: Owner ruling, then align both texts. (a) Promote: state in the `CausalMessages` rustdoc that concurrent messages are delivered in a deterministic order that is a function of the set alone (causal rank, then canonical version bytes), identical on every replica holding the same set; keep both tests as pins of that contract. (b) Demote: keep the public doc, and re-label the two tests' docs as pins of the engine's current staging order (an internal-entry check, documented at the site as a deliberate decision), or rewrite `delivery_order_is_replica_independent` to assert `assert_causal` on each side plus set-equality and delete the strict `(rank, bytes)` assertion at 146-153. Acceptance: the rustdoc on `CausalMessages` and the two test docs state the same order contract, or the tests declare themselves internal-order pins at the site.

### tests-observation-4: Ragged doc wraps left by the first-sentence split
- Where: tests/causal.rs:184-189 (related: tests/causal.rs:214-222, tests/causal.rs:454-459, tests/changes.rs:165-170, tests/listen.rs:164-169, tests/listen.rs:464-473, tests/listen.rs:612-620, tests/observe.rs:406-411, tests/party_conservation.rs:124-129, tests/party_conservation.rs:153-158, tests/party_conservation.rs:200-205, tests/party_conservation.rs:213-219, tests/party_conservation.rs:297-306, tests/party_conservation.rs:339-346)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`git blame` attributes the inserted blank `///` and fragment line at changes.rs:165-167 and party_conservation.rs:124-126 to dfd19c447b, the continuation lines to older commits)
- Seen by: structure-prose [12]; refutation: confirmed; history: no rationale found (mechanical residue of the doclint-driven split)
- Owner-gated: no

Commit dfd19c447b inserted a blank `///` after each doc's first sentence to satisfy doclint's summary rule but did not re-wrap the following paragraph, leaving two-to-four-word fragments on their own lines at fourteen sites. The rendered output is unaffected; the source, which is read far more often, is not.

Evidence:

    184	/// Causal order holds *across* passes, not just within one: messages
    185	/// delivered live (pass by pass, interleaved with sends and gossip) never
    186	/// invert against earlier deliveries.
    187	///
    188	/// Order holds because a later pass can never
    189	/// contain a causal predecessor of an earlier pass's message.

    (tests/changes.rs)
    167	/// A fresh
    168	/// signal's first step ticks, commits between steps coalesce into one tick,

Resolution: Re-wrap the second paragraph at each listed site to the file's column. Acceptance: no `///` line under about 30 characters in the partition is followed by a `///` line continuing its sentence.

### tests-observation-5: The causal interleaving strategy never redacts a remote or gossip-learned message; the unordered interleaving has no gossip
- Where: tests/causal.rs:359-386 (related: tests/causal.rs:405-423, tests/causal.rs:258-301, tests/listen.rs:528-552, tests/common/sim.rs:571)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read `Op`, `arb_ops`, and the executor loop: `sent` is pushed only in `SendA`; listen.rs's `Op` is Send, Redact, Drain)
- Seen by: blind-spots [28]; refutation: confirmed; history: no rationale found for causal.rs's redact scope; listen.rs's alphabet matches the deleted plan's specification, and the multi-peer regime is covered by `run_observers` in tests/common/sim.rs
- Owner-gated: no

`Op::Redact` indexes only `sent`, the versions A itself created; B never redacts, and A never redacts a message it learned from B. The region where deletion honoring interacts with the staged backlog (B redacts a message A has ingested and staged but not delivered, then gossip sheds it from A's tree before the pop) is unreachable, as is A redacting a learned message mid-backlog. The point test `staged_then_redacted_is_still_delivered` covers only the local-redaction variant. In listen.rs, `exactly_once_under_interleaving` has no gossip op, so the unordered observer's exactly-once claim under gossip-learned content plus redaction is sampled only by disruption.rs's concurrent observers. White-box worst-case construction: the shape that stresses the staging assumption belongs in the roster.

Evidence:

    366	    /// Redact the `idx % sent`-th message sent at `a` so far (dropped
    367	    /// if none).
    368	    Redact(usize),
    ...
    415	                Op::Redact(idx) => {
    416	                    if !sent.is_empty() {
    417	                        a.redact(&sent[idx % sent.len()]);
    418	                    }
    419	                }

Resolution: Extend causal.rs's alphabet with `RedactB(usize)` (B redacts one of its own sends, tracked in `sent_b`) and `RedactLearned(usize)` (A redacts the idx-th version of `a.snapshot()`), keeping the existing assertions. For listen.rs, either add `Gossip` and `SendB` ops mirroring causal.rs or state in the testdoc that the multi-peer regime is covered by `disruption.rs`. Acceptance: a default run's shrunk Debug output shows redactions of B-originated and A-learned versions; a point test pins the staged-then-shed shape (A stages both, B redacts one, gossip, drain).
Construction: In causal.rs add a point test: `a` seeds, `b = bootstrap_fork(&a)`, `b.send(1); b.send(2)`, `wire_gossip(&a,&b)`, `let mut obs = a.causal_messages(); step(&mut obs)` (ingests both, delivers one), `b.redact(&staged_version); wire_gossip(&a,&b); drain(&mut obs)`. The current strategy cannot generate this sequence; the point test states what the expected delivery is (the staged message is delivered once, per the ingest-liveness contract) and fails if the pop path re-reads the tree.

### tests-observation-6: `any::<usize>()` modulo a generated length where `prop::sample::Index` shrinks better
- Where: tests/causal.rs:464-477 (related: tests/causal.rs:539, tests/causal.rs:557, tests/listen.rs:625, tests/listen.rs:642, src/tree/tests.rs:254)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read the three sites; `prop::sample::Index` is already the crate's idiom in src/tree/tests.rs)
- Seen by: structure-prose [22]; refutation: confirmed, reframed (the generation-time distinction is not load-bearing: `Index::index(n)` takes its bound at execution time too); history: no rationale found
- Owner-gated: no

`taken in any::<usize>()` reduced by `taken % (phase_one.len() + 1)` shrinks by jumping around the modulus; `prop::sample::Index` shrinks the index monotonically toward zero, yielding smaller counterexamples in committed seed files. party_conservation.rs documents the modulo form as the sim.rs idiom for fleet indices; these three sites have no such note.

Evidence:

    464	        taken in any::<usize>(),
    ...
    477	            for _ in 0..(taken % (phase_one.len() + 1)) {

Resolution: `taken in any::<prop::sample::Index>()` and `taken.index(phase_one.len() + 1)` at causal.rs:464/477, 539/557 and listen.rs:625/642. Acceptance: no `taken % (... + 1)` remains in the two observer suites.

### tests-observation-7: `Changes` has only point tests, and never issues a no-op commit with an observer attached
- Where: tests/changes.rs:1-6 (related: src/rumors.rs:176-177, src/rumors.rs:224, src/batch.rs:129-144, src/tree/tests.rs:1123-1129)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read changes.rs in full; read `Batch::commit` and the tree-tier changed-flag property doc)
- Seen by: blind-spots [30]; refutation: reframed (the changed flag is already pinned `false` on empty and unheld-forget batches at the tree tier, `act_changed_flag_tracks_the_root_hash`; what is missing is the end-to-end pin that the flag suppresses the wakeup, and a family-level property); history: no rationale found
- Owner-gated: no

The module doc states a family claim ("exactly one coalesced tick per observed frontier advance (however many commits that was)") and the suite pins it with eight fixed sequences and no property. Two documented no-op commits are never issued with an observer attached: a redact of an unheld version (src/rumors.rs:176-177) and `send_all` of an empty iterator (src/rumors.rs:224, "wakes no observer"). `Batch::commit` passes `act`'s flag straight to `send_if_modified` (src/batch.rs:140-143), so a `commit` that returned `true` unconditionally would escape this suite. AGENTS.md: when the claim is a family, state it as a proptest invariant.

Evidence:

    3	//! Pins the contract stated on the type: an immediate first yield, exactly
    4	//! one coalesced tick per observed frontier advance (however many commits
    5	//! that was), ticks for every kind of commit — send, redact, and a join
    6	//! learned by gossip — and a clean end once the set closes.

    (src/rumors.rs)
    224	    /// it. An empty iterator commits nothing and wakes no observer.

Resolution: Add a proptest over `Vec<{Send(u64), SendAll(0..=3 values), RedactHeld(idx), RedactUnheld, Poll}>` that records `latest()` at each Poll and asserts `try_next() == Tick` iff `latest` differs from the last reported one, `Quiet` otherwise. Add two point tests: `redact` of a version the set never held and `send_all(std::iter::empty::<u64>())` both leave a reported signal `Quiet`. Acceptance: a mutant making `Batch::commit`'s closure return `true` unconditionally fails the new tests.
Construction: In tests/changes.rs, `let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors(); let mut changes = rumors.changes(); assert_eq!(changes.next().now_or_never(), Some(Some(()))); rumors.send_all(std::iter::empty::<u64>()).unwrap(); assert_eq!(changes.next().now_or_never(), None);` and the same with `rumors.redact(&Version::new())`. Today nothing in the suite makes either call with an observer attached.

### tests-observation-8: changes.rs runs under `#[pollster::test]`, bypassing the closed-world poller, and four of its async tests never await
- Where: tests/changes.rs:14-19 (related: tests/changes.rs:29-54, tests/changes.rs:58-69, tests/changes.rs:73-85, tests/changes.rs:103-137, tests/changes.rs:142-154, tests/changes.rs:158-163, tests/listen.rs:309-314, tests/common/wire.rs:40-42, .config/nextest.toml:1-5)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`grep -n '\.await' tests/changes.rs` gives only lines 76, 83, 106, 123, 162; read `block_on` in wire.rs and the nextest.toml design statement)
- Seen by: structure-prose [13], api-economics [43]; refutation: confirmed; history: no rationale found (the swap from `tokio::test` to pollster landed in 83edcd944, a WIP commit with no message body, the same commit that introduced `run_to_quiescence`)
- Owner-gated: no

Every other suite in the partition drives sessions through `common::wire::block_on`, which is `run_to_quiescence(..).expect(..)`: a wire stall fails at the stalled poll with a verdict. changes.rs runs seven tests under pollster's executor instead, so a stall in `bootstrap_fork_async`/`wire_gossip_async` would park the thread until nextest's 180 s kill with no diagnosis. Four of those tests (`first_poll_yields_immediately`, `one_tick_per_observed_commit`, `unpolled_commits_coalesce_to_one_tick`, `set_closure_ends_the_stream`) have no `.await` at all, and `observer_does_not_block_peer_reclaim` awaits `try_into_peer()` so that a regression (an observer counted against quiescence) fails by hang, where listen.rs:309-314 makes the identical claim with `.now_or_never().expect(..)` and fails crisply. The sync `bootstrap_fork`/`wire_gossip` already serve listen.rs and causal.rs.

Evidence:

    14	use crate::common::wire::{bootstrap_fork_async, wire_gossip_async};
    ...
    18	#[pollster::test]
    19	async fn first_poll_yields_immediately() {

    158	#[pollster::test]
    159	async fn observer_does_not_block_peer_reclaim() {
    160	    let rumors: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    161	    let _changes = rumors.changes();
    162	    assert!(rumors.try_into_peer().await.is_some());
    163	}

Resolution: Make all seven tests plain `#[test] fn`, call `bootstrap_fork`/`wire_gossip`, and in the reclaim test use `rumors.try_into_peer().now_or_never().expect("an observer does not count against quiescence")`. Drop the pollster import from this file (handshake.rs and gossip_when.rs keep the dev-dependency unless they follow). This also removes changes.rs from the `testdoc` hole in tests-observation-37. Acceptance: `grep -c pollster tests/changes.rs` is 0; the seven tests pass unchanged in behavior.

### tests-observation-9: The received-side mirror check lacks the handler-count equality its sent-side twin has
- Where: tests/observe.rs:205-227 (related: tests/observe.rs:173-200, tests/observe.rs:123-143)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read both helpers; only `assert_mirrors` has the length equality)
- Seen by: blind-spots [29]; refutation: confirmed; history: no rationale found (both helpers date from the hook's first commit)
- Owner-gated: no

`assert_mirrors` asserts `sent.len() == capture.streams.len()` before matching per index, but `assert_received_mirrors_remote` only iterates the remote's captured streams and finds a matching received handler for each; a phantom received data-stream handler at an index the peer never spoke passes. `SessionRecord::data` rejects duplicate indices per direction, so only the extra-index case escapes. The module doc promises "one handler per directed data stream" in both directions; the received direction is the dual and should carry the same totality check.

Evidence:

    180	    assert_eq!(
    181	        sent.len(),
    182	        capture.streams.len(),
    183	        "{side}: one sent data handler per opened transport stream"
    184	    );
    ...
    215	    let received = local.data(Direction::Received);
    216	    for blob in &remote_capture.streams {
    217	        let ((_, index), label_len) = stream_label(blob);
    218	        let Some((_, _, bytes)) = received.iter().find(|(i, ..)| *i == index) else {
    219	            panic!("{side}: no received handler for the peer's data stream {index}");

Resolution: Add `assert_eq!(received.len(), remote_capture.streams.len(), "{side}: one received data handler per stream the peer opened");` before the loop at 216. Acceptance: a mutant that invokes a received data-stream handler for an index the peer never opened fails `assert_received_mirrors_remote`.

### tests-observation-10: `assert_election`'s `opened` predicate is asymmetric in A and B
- Where: tests/observe.rs:235-237 (related: tests/observe.rs:359-363, tests/observe.rs:478-482)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read the helper and both call sites: `assert_received_mirrors_remote` runs before `assert_election` at each, binding A's received streams to B's sent ones)
- Seen by: structure-prose [19], api-economics [53]; refutation: confirmed; history: no rationale found (written in 40b1e96ae and never touched)
- Owner-gated: no

The third disjunct consults A's received streams but not B's; over the lossless link A.received equals B.sent, which the preceding `assert_received_mirrors_remote` has already checked, so the disjunct is redundant with the second and the asymmetry reads as accidental. A reviewer stops to work out why one received side matters and the other does not.

Evidence:

    235	    let opened = !a.data(Direction::Sent).is_empty()
    236	        || !b.data(Direction::Sent).is_empty()
    237	        || !a.data(Direction::Received).is_empty();

Resolution: Reduce to the two `Sent` disjuncts. Acceptance: `opened` is symmetric in A and B.

### tests-observation-11: `forked()` re-derives the bootstrap fork without the control-drain assertion because the harness cannot attach a builder observer
- Where: tests/observe.rs:275-289 (related: tests/common/wire.rs:244-264, tests/observe.rs:256-270)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (read wire.rs in full: `bootstrap_fork_configured` is private, constructs `Peer::<T>::bootstrap()` internally, and calls `assert_control_drained` at 262; `forked()` uses `rumors::link::memory()` and no drain check)
- Seen by: api-economics [44]; refutation: confirmed; history: no rationale found
- Owner-gated: no

`forked()` reimplements the bootstrap fork to attach an `Observer` on the `Bootstrap` builder, which no `tests/common/wire.rs` entry allows. In doing so it uses the default-capacity `memory()` link rather than `LINK_BUF`, pins no window, and omits `assert_control_drained`, the post-condition every other bootstrap in the partition checks (wire.rs:65-79 explains why a leftover control byte must fail at the session that caused it).

Evidence:

    275	fn forked(observer: Option<&Arc<Recording>>, parent: &Rumors<Vec<u8>>) -> Rumors<Vec<u8>> {
    276	    let (mut near, mut far) = rumors::link::memory();
    277	    let serve = parent.clone();
    278	    let mut bootstrap = Peer::<Vec<u8>>::bootstrap();
    279	    if let Some(observer) = observer {
    280	        bootstrap = bootstrap.observe(observer.clone());
    281	    }
    282	    block_on(async {
    283	        let (peer, served) = tokio::join!(bootstrap.join(&mut near), serve.gossip(&mut far),);

Resolution: Add `bootstrap_fork_with(parent, configure: impl FnOnce(Bootstrap<T>) -> Bootstrap<T>)` (or an `_async` core taking the builder) to `tests/common/wire.rs`, routed through `bootstrap_fork_configured` so `LINK_BUF`, the window choice, and the drain assertion apply; have `forked()` call it. Acceptance: observe.rs contains no direct `Peer::bootstrap()` call in `forked()`; the join session's control stream is asserted drained.

### tests-observation-12: Helpers duplicated verbatim between a suite and its sibling or `tests/common`: `corpora` and `gossip_pair`
- Where: tests/observe.rs:291-302 (related: tests/wire_legibility.rs:104-116, tests/session_stats.rs:26-41, tests/common/wire.rs:144-158)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read all four bodies and compared; `git log -S'fn gossip_pair'` gives 7626543e (2026-07-24) for the session_stats copy and 0d48b153 (2026-08-13) for the shared one)
- Seen by: structure-prose [3] [4], api-economics [45]; refutation: confirmed; history: no rationale found for either copy (observe.rs copied wire_legibility.rs one hour after it landed, documenting the copy as "mirroring"; the shared `gossip_pair_async` was added beside the older private one rather than replacing it)
- Owner-gated: no

observe.rs's `corpora()` is byte-identical to tests/wire_legibility.rs:108-116 (payload `vec(any::<u8>(), 0..48)`, three sides `0..12`, same `#[allow]`), and its doc promises the two "mirror" each other: a hand-maintained invariant a shared definition makes structural. session_stats.rs's `gossip_pair` is line-for-line `common::wire::gossip_pair_async` (same capacity, same `tokio::join!`, same expects, same `assert_control_drained`); the `assert_control_drained` and serde imports at session_stats.rs:26-29 exist only to serve the copy.

Evidence:

    291	/// Arbitrary payload corpora, mirroring the wire-legibility suite's
    292	/// shape: enough variety to drive matches, queries, empty queries, and
    293	/// batched supply runs, small enough for many full sessions.
    294	#[allow(clippy::type_complexity)]
    295	fn corpora() -> impl Strategy<Value = (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>)> {
    296	    let payload = vec(any::<u8>(), 0..48);

    (tests/session_stats.rs)
    32	async fn gossip_pair<T>(a: &Rumors<T>, b: &Rumors<T>) -> (Gossiped, Gossiped)
    ...
    36	    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);
    37	    let (a_out, b_out) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
    38	    let pair = (a_out.expect("gossip A"), b_out.expect("gossip B"));
    39	    assert_control_drained(a_link, b_link);
    40	    pair
    41	}

Resolution: Move `corpora` into `tests/common/gossip_snapshot.rs` (the capture harness both suites already import) and import it in observe.rs and wire_legibility.rs, dropping the "mirroring" sentence. Delete `session_stats::gossip_pair` and its serde imports; replace its six call sites (58, 83, 85, 108, 128, 320) with `gossip_pair_async`; drop `assert_control_drained` from the line-26 import. Acceptance: `grep -rn 'fn corpora\|fn gossip_pair' tests/*.rs` is empty.

### tests-observation-13: The `Protocol::V2` assertion is vacuous with a one-variant enum
- Where: tests/observe.rs:350-352 (related: tests/observe.rs:316, src/protocol.rs:12-19, src/observe.rs:131-136)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (read src/protocol.rs in full: `#[non_exhaustive] pub enum Protocol { #[default] V2 = 2 }`)
- Seen by: structure-prose [6], api-economics [52]; refutation: confirmed, severity down to nit; history: deliberate-but-expired (written when two variants existed; V1 retired in 368da2a5; the retirement note deliberately keeps the enum and `SessionInfo.protocol` as wire vocabulary but names only src/observe.rs's dialect guard for removal, not this line)
- Owner-gated: no: dropping the assertion does not touch the kept field

`Protocol` has exactly one variant, so `session.info.protocol == Protocol::V2` cannot be false; the assertion and the doc clause "carries the right kind and protocol" at 316 check nothing. A check nothing can fail is decoration. Whether `SessionInfo.protocol` itself should survive is a production API question recorded in the retirement note and outside this partition.

Evidence:

    350	            prop_assert_eq!(session.info.kind, SessionKind::Gossip, "{}", side);
    351	            prop_assert_eq!(session.info.protocol, rumors::Protocol::V2, "{}", side);

Resolution: Drop line 351 and the words "and protocol" from the doc at 316; if the field's retention is deliberate, one non-proptest pin suffices. Acceptance: `grep -n 'Protocol::V2' tests/observe.rs` is empty.

### tests-observation-14: `bootstrap_sessions_are_observed` omits the `assert_election` check its retire sibling makes
- Where: tests/observe.rs:436-444 (related: tests/observe.rs:482, tests/observe.rs:406-411, src/tree/mirror/streaming/remote/proxy/start.rs:356-376, src/peer/gossip.rs:1152-1165)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read start.rs: the election fires whenever greeting versions differ, before any data stream opens; read the provider path's handshake at gossip.rs:1152; in the fixture the provider holds one message and the newcomer is at genesis)
- Seen by: structure-prose [10]; refutation: confirmed; history: no rationale found (the asymmetry dates from the hook's first commit, whose message lists the election check only for the gossip proptest)
- Owner-gated: no

The retire pairing ends with `assert_election(&absorber_session, &retiree_session)`; the bootstrap pairing does not. A bootstrap session runs the same handshake and descent, and with a seeded provider facing an empty newcomer the greeting versions differ, so both sides elect and data streams open; the election and speaker agreement are checkable here too. The two fixed pairings exist to cover the session kinds end to end (module doc 16-18); the election is part of that view.

Evidence:

    438	    assert_eq!(provider_session.info.kind, SessionKind::Gossip);
    439	    assert_eq!(newcomer_session.info.kind, SessionKind::Bootstrap);
    440	    assert_mirrors("provider", &provider_session, &provider_capture);
    441	    assert_mirrors("newcomer", &newcomer_session, &newcomer_capture);
    442	    assert_received_mirrors_remote("provider", &provider_session, &newcomer_capture);
    443	    assert_received_mirrors_remote("newcomer", &newcomer_session, &provider_capture);
    444	}

Resolution: Add `assert_election(&provider_session, &newcomer_session);` and mention the election in the doc at 406-411; if the bootstrap pairing deliberately skips it, say why at the site. Acceptance: both fixed-pairing tests call `assert_election`, or the bootstrap test's doc states why the election is out of scope.

### tests-observation-15: Fourteen testdocs in listen.rs open with `§6.n` tags from a plan file deleted from the tree
- Where: tests/listen.rs:73-75 (related: tests/listen.rs:109, 137, 164, 215, 241, 276, 298, 330, 357, 380, 464, 555, 612)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -n '§' tests/listen.rs` returns exactly these fourteen lines; `git log -S'§6.1'` gives 040a0d042 "tests/listen.rs: the Messages observer contract (plan §6)", whose body cites plans/broadcast-listen.md; `git log --diff-filter=D -- plans/broadcast-listen.md` gives 12f9b85e9 "Remove old and intermediate artifacts")
- Seen by: structure-prose [0], api-economics [46]; refutation: confirmed (fourteen sites, not thirteen); history: contradicts hard rule (one correction: the repeated 6.6 and 6.9 tags mirror the plan's own "Negative control" and "Variant" sub-items, so they are orphaned, not disordered)
- Owner-gated: no

The tags are section numbers of `plans/broadcast-listen.md`, which the suite's own commit cites and which was deleted two days later. Nothing in the tree resolves them; they are opaque roster IDs to every reader today, and they break the doc's first sentence in a module listing. Hard rules: no design-document citations from code; no opaque roster IDs. The English titles that follow each tag already carry the meaning.

Evidence:

    73	/// §6.1 Genesis replay: a from-genesis observer on a populated set yields
    74	/// exactly the live set, each message once, then goes quiet; after the
    75	/// completed pass its checkpoint dominates every observed version.

Resolution: Delete the `§6.n ` prefix at each of the fourteen doc comments, keeping the English title; the parentheticals "(retire variant)" and "(negative control)" may stay as plain words ("Retire variant of termination: ...", "Negative control: ..."). Acceptance: `grep -c '§' tests/listen.rs` prints 0 and every affected doc still opens with its English title.

### tests-observation-16: Braced single-statement blocks left by the batch-closure to `send_all` rewrite
- Where: tests/listen.rs:79-81 (related: tests/listen.rs:629-632, tests/listen.rs:653-656, tests/session_stats.rs:262-268, tests/session_stats.rs:309-316, tests/stale_floor.rs:35-37)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 212c6914 -- tests/listen.rs` shows the block previously held a multi-line `.batch(|batch| { .. }).expect(..)` chain)
- Seen by: structure-prose [5], api-economics [48]; refutation: confirmed; history: deliberate-but-expired (at 040a0d042 the braces scoped a Drop-committing batch guard; ce27df86 turned `Batch` into a closure scope, deleting that purpose; 212c6914 carried the braces forward around one `send_all`)
- Owner-gated: no

Six sites brace a single `send_all(..).unwrap()` in an otherwise empty block; inside `proptest!` bodies, which rustfmt cannot reach, the rewrite also left the receiver on its own line. A scoped block signals a lifetime or borrow that matters; here none does, and the reader stops to look for one.

Evidence:

    79	    {
    80	        rumors.send_all(0..8u64).unwrap();
    81	    }

    (tests/session_stats.rs)
    309	            {
    310	                a
    311	                    .send_all(a_sends.iter().copied()).unwrap();
    312	            }

Resolution: Unwrap the braces and re-flow to one line at listen.rs:79-81, 629-632, 653-656; session_stats.rs:262-268, 309-316; stale_floor.rs:35-37. Acceptance: none of the six sites has a bare `{` preceding a lone `send_all`; `just fmt-check` still passes.

### tests-observation-17: `retire_ends_the_observer`'s doc claims the final drain includes what the session learned; the session learns nothing
- Where: tests/listen.rs:241-243 (related: tests/listen.rs:246-248, tests/listen.rs:268-273, src/peer/gossip.rs:416-419)
- Class / severity / confidence: test-quality / medium / high
- Provenance: assessed (read 241-274: `survivor` is seeded and never sends; `retiree.send(7)` precedes the observer's creation at 250; the sole content assertion is that 7 appears)
- Seen by: structure-prose [9], blind-spots [26]; refutation: confirmed; history: no rationale found (the doc's second sentence restates the production promise at gossip.rs:416-419; the body has never exercised it since 040a0d042)
- Owner-gated: no

The doc's second sentence states the write-back-before-end ordering `retire` promises ("observers of a retiring set ... drain the *reconciled* final state — everything the session learned included — before they end", src/peer/gossip.rs:416-419). In the body the survivor holds nothing, so the session teaches the retiree nothing, and a from-genesis observer delivers message 7 whether or not the write-back precedes termination. An implementation that ended the observer on the pre-session state passes. AGENTS.md: the doc comment states the behavior the test protects; an inaccurate testdoc is a bug in the test, and this one states a production promise with no failing implementation in the suite.

Evidence:

    241	/// §6.9 (retire variant): retiring the rumor set ends its observers. The
    242	/// retire session's write-back lands the reconciled state first, so the
    243	/// observer's final drain includes everything the session learned.
    ...
    246	    let survivor = Peer::<u64>::seed().sync_window_floor().into_rumors();
    247	    let retiree = bootstrap_fork(&survivor);
    248	    retiree.send(7).unwrap();
    ...
    270	    assert!(
    271	        items.iter().any(|(_, m)| *m == 7),
    272	        "the final drain delivered the retiree's own message"
    273	    );

Resolution: After the observer subscribes (line 250) and before the retirement, `survivor.send(8).unwrap();`; after the drain assert that the items contain both 7 and 8, with 8 named as the message the session learned. Acceptance: ending the retiree's observer ahead of the write-back (or dropping learned content from it) fails the test on message 8.
Construction: Apply the two-line change above and temporarily swap the order of the write-back and the observer-ending step in `retire_inner`; the test must fail on 8. Without the change, the same swap passes.

### tests-observation-18: Ghost vocabulary from the dissolved lending forms: "borrowed faces", "lent out", `lent_borrows_do_not_block_senders`
- Where: tests/listen.rs:330-334 (related: tests/listen.rs:28, tests/listen.rs:339-340, tests/listen.rs:352-353, tests/causal.rs:26, src/rumors/unordered.rs:192, src/rumors/causal.rs:152)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git show cd7c09db` "observers: dissolve the lending forms" removed `borrow_next` and the lending `TryNext`; both observers' `Stream::Item` is the owned `(Version, Arc<T>)`; `grep -n 'lent\|borrow' tests/listen.rs tests/causal.rs` hits only these sites plus the unrelated `redactions_are_honored_silently`)
- Seen by: structure-prose [1]; refutation: confirmed; history: contradicts hard rule (cd7c09db rewrote the `borrow_next` mentions and one inline comment in these suites but left the `Step` docs, the test name, its doc sentence, the `lent`/`lent_value` bindings, and the assert message: an incomplete sweep inside the removing commit)
- Owner-gated: no

Nothing in the API these tests exercise borrows or lends: `step` copies out of an owned item, and the test holds an owned `Arc`. The `Step` doc in both suites still says "with the borrowed faces cloned out", and this test is named for lent borrows with a doc about an item "still lent out". A reader looks for the lending API to understand what "lent out" means and finds none. Hard rule: nothing refers to code that no longer exists.

Evidence:

    28	/// One observer step, with the borrowed faces cloned out.

    330	/// §6.12 Non-blocking observer: an observer mid-pass — its most recent item
    331	/// still lent out — holds no lock, so sends on the set proceed and the
    332	/// observer sees their effects on its next passes.
    333	#[test]
    334	fn lent_borrows_do_not_block_senders() {

Resolution: Rename to `mid_pass_observer_does_not_block_senders`; reword the doc to "an observer mid-pass, one of the pass's items handed out and still held, holds no lock..."; rename `lent`/`lent_value` to `held`/`held_value` and the assert message to match. In both suites change the `Step` doc (causal.rs:26, listen.rs:28) to "One observer step, with the item's payload copied out of its `Arc`." (moot if tests-observation-2 lands). Acceptance: `grep -n 'lent\|borrow' tests/listen.rs tests/causal.rs` matches only `redactions_are_honored_silently`.

### tests-observation-19: Economy and moral register in testdocs: "earned"/"earns" for checkpoints, "honest sessions"
- Where: tests/listen.rs:357-359 (related: tests/listen.rs:376, tests/listen.rs:380, tests/listen.rs:391, tests/session_overlap.rs:61-62)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (`grep -n earn tests/listen.rs`; `grep -n honest tests/session_overlap.rs`; the observer rustdoc in src/rumors/unordered.rs and src/rumors.rs does not use "earn")
- Seen by: refutation pass (new items 1 and 2); refutation: raised; history: not examined
- Owner-gated: no

"A checkpoint earned by a completed pass ... earns an equal checkpoint" transplants an economy register where none exists (a pass produces a checkpoint; nothing is spent or won), and the vocabulary is test-local: the observer's own rustdoc says a completed pass's frontier. "Two honest sessions" in session_overlap.rs carries nothing under the honest-peer model of record, where every session is honest, and reads as moralized code.

Evidence:

    357	/// §6.7 Checkpoint round-trip: a checkpoint earned by a completed pass, fed
    358	/// to a fresh `unordered_messages_since` on an unchanged set, observes
    359	/// nothing and earns an equal checkpoint.

    (tests/session_overlap.rs)
    61	/// No interleaving of two honest sessions may lose a message nobody
    62	/// redacted.

Resolution: "a checkpoint from a completed pass ... yields an equal checkpoint" (357-359, 376, 380, 391); "No interleaving of two sessions may lose a message nobody redacted." Acceptance: `grep -n 'earn\|honest' tests/listen.rs tests/session_overlap.rs` matches nothing but "learned".

### tests-observation-20: `checkpoint_resume_loses_nothing`'s doc states a mid-pass bound the body admits is tautological, and neither run is checked for internal duplicates
- Where: tests/listen.rs:616-620 (related: tests/listen.rs:679-692, tests/causal.rs:454-459, tests/causal.rs:510-520)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read 612-693: `first_versions`/`second_versions` are `BTreeSet`s, so a within-run duplicate collapses unchecked; the comment at 691-692 says the mid-pass bound holds by construction)
- Seen by: api-economics [50]; refutation: confirmed; history: deliberate-and-holds for the omitted assertion (the author wrote the 691-692 comment at birth); the doc sentence was carried from the plan's wording
- Owner-gated: no

The doc promises that mid-pass "re-deliveries are permitted but only for messages the interrupted pass already delivered"; the body computes `redelivered = first ∩ second`, and its own comment notes `redelivered ⊆ first_versions` "holds by construction", so the stated property is not tested. Neither run is checked for duplicates within itself, so a `_since(checkpoint)` observer re-yielding one message twice passes here. The doc should name only checked claims; the substantive mid-pass claim worth pinning is that each run is itself exactly-once.

Evidence:

    616	    /// If the
    617	    /// stop fell *mid-pass*, re-deliveries are permitted but only for
    618	    /// messages the interrupted pass already delivered (at-least-once); if
    619	    /// the observer had *completed* its pass, nothing from it is
    620	    /// re-delivered (exactly-once across completed passes).
    ...
    691	        // (Mid-pass, `redelivered ⊆ first_versions` holds by construction;
    692	        // the loss-freedom assertion above is the substantive check.)

Resolution: Reword the doc to the two checked claims (the union covers the final live set; nothing re-fires after a completed pass) and add `prop_assert_eq!(first_versions.len(), first_run.len())` and `prop_assert_eq!(second_versions.len(), second_run.len())` so each run is duplicate-free; delete the 691-692 comment. Mirror the duplicate check in causal.rs:510-520. Acceptance: the doc names only checked properties; a within-run duplicate fails the test.

### tests-observation-21: Import-path inconsistencies across the partition
- Where: tests/session_overlap.rs:20-30 (related: tests/stale_floor.rs:24, tests/shadow_validity.rs:41, tests/session_stats.rs:26-30, tests/bootstrap.rs:22-24, tests/common/wire.rs:20-22, tests/common/overlap.rs:44-46, tests/common/schedule/executor.rs:22-24)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rln '^use common::' tests/*.rs` gives session_overlap.rs and stale_floor.rs; read the cited lines)
- Seen by: structure-prose [17], api-economics [54]; refutation: confirmed (39 suites use `crate::common`, 2 use `common`); history: no rationale found
- Owner-gated: no

Two suites write `use common::...` where the other thirty-nine write `use crate::common::...`; session_overlap.rs imports `Rumors` but spells `rumors::Peer::seed()` at line 30; shadow_validity.rs:41 writes `std::ops::RangeInclusive<usize>` in a const type; session_stats.rs:28-30 (and the listed common modules) glue the two serde imports to the following item's doc comment with no blank line. One import convention per suite lets `grep` find every consumer of a common module.

Evidence:

    20	use common::oracle::{readout, readout_multiset};
    21	use common::overlap::{self, arb_overlap_schedule, execute_overlap_and_quiesce};
    22	use common::wire::{bootstrap_fork, wire_gossip};
    23	use proptest::prelude::*;
    24	use rumors::{Rumors, Version};
    ...
    30	    let a = rumors::Peer::seed().into_rumors();

    (tests/session_stats.rs)
    28	use serde::Serialize;
    29	use serde::de::DeserializeOwned;
    30	/// Run one gossip session between two handles over an in-memory link,

Resolution: Switch the two files to `use crate::common::`; import `Peer` in session_overlap.rs; `use std::ops::RangeInclusive;` in shadow_validity.rs; move the serde imports into the external-crate group and add the blank line before the doc comment at each listed site. Acceptance: `grep -rln '^use common::' tests/*.rs` is empty; the three qualified-path sites are gone.

### tests-observation-22: The deterministic overlap sweep seeds `a` at the default window while every other fixture in the partition pins the floor, with no stated reason
- Where: tests/session_overlap.rs:29-37 (related: tests/common/overlap.rs:209, tests/common/wire.rs:213-218, tests/session_overlap.rs:84-93)
- Class / severity / confidence: test-quality / nit / medium
- Provenance: verified (read line 30; overlap.rs:209 pins the floor for the generated fleet; `bootstrap_fork_async` pins `WindowChoice::Floor`; `git show 0b353ffc3:tests/session_overlap.rs` shows line 30 unchanged since the file's birth and the commit message says nothing about windows)
- Seen by: blind-spots [38], api-economics [51]; refutation: confirmed; history: no rationale found (the pin-the-floor convention landed six days before this suite; a plausible unstated reason is sweep size)
- Owner-gated: no (the choice is a fixture decision, but only the owner knows whether the default window was needed to reproduce the incident; see open questions)

`converged_trio` seeds `a` with `rumors::Peer::seed().into_rumors()` while `bootstrap_fork` pins `b` and `c` at the floor, so the swept and calibrated sessions run asymmetric windows. Running the sweep at the default and the generated family at the floor may be useful diversity, but nothing says it is deliberate, so a reader cannot tell it from an omission. Comments state what the code cannot show: whether a choice is a decision.

Evidence:

    29	fn converged_trio(n: u64) -> (Rumors<u64>, Rumors<u64>, Rumors<u64>) {
    30	    let a = rumors::Peer::seed().into_rumors();

Resolution: Either add `.sync_window_floor()` for uniformity or add one sentence to `converged_trio`'s doc naming the intent (the sweep exercises the production default on the seed while its forks run the floor). Acceptance: the doc states the window choice or the seed line matches the suite's convention.

### tests-observation-23: The overlap sweep and the pincer motif both run one orientation; the dual is reached only by the random soup
- Where: tests/session_overlap.rs:61-134 (related: tests/session_overlap.rs:113-121, tests/common/overlap.rs:480-500)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read the sweep body and `Pincer::choices`: both open the converged pair, run the mutating session inside the park, then close)
- Seen by: blind-spots [31]; refutation: confirmed; history: no rationale found (both instruments were built in the discovering incident's orientation)
- Owner-gated: no

The sweep opens the innocent session A<->C, parks it after `n` polls, runs the redacting session A<->B to completion, then finishes A<->C. `Pincer::choices` emits the same orientation. The mirror image, the redacting session forked first and installing last over the innocent one, is a distinct install-time interleaving (its merged frontier carries the redaction tick while A's current tree has been re-joined with C's fork-time copy) and is sampled only when the soup happens to produce it. Doctrine asks for the dual family per operation pair; the module doc names the symptom as orientation-independent.

Evidence:

    113	            let s2 = {
    114	                let mut s2 = overlap::open(&a, &c);
    115	                s2.step(n);
    116	                s2
    117	            };
    118	            // ...S1 (A <-> B) installs the honored redaction at A...
    119	            wire_gossip(&a, &b);
    120	            // ...and S2 resumes and installs after it.
    121	            s2.finish();

Resolution: Add a second arm to the sweep with A<->B opened first and parked at `n`, `wire_gossip(&a, &c)` run whole, then the parked session finished, against the same `expected`. In `Pincer::choices`, draw an orientation bit and emit the mirrored sequence (Open x<->w mutating, Step, Gossip x<->y, Close) when set. Acceptance: both orientations are swept over every (target, n) pair; the pincer strategy's Debug output shows both orientations in a default run.
Construction: Duplicate lines 110-121 with the roles of `b` and `c` swapped in the open/park/finish structure (open A<->B after `b.redact`, park, `wire_gossip(&a, &c)`, finish) and run the sweep; today no committed test executes that sequence deterministically.

### tests-observation-24: `LABEL_LEN = 2` transcribes a label length the code computes, and is true only for session epochs below 24
- Where: tests/session_stats.rs:246-249 (related: tests/session_stats.rs:277-288, src/tree/mirror/streaming/remote/streams.rs:62-67, src/link.rs:390-391, src/link.rs:428, src/tree/mirror/streaming/remote/codec/capture.rs:117-121, tests/observe.rs:186, tests/observe.rs:217)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read `label()`: two CBOR uint heads sized by `cbor::head_len`; a fresh link starts at epoch 0 and advances one per session; `stream_label` returns the parsed label length and observe.rs uses it at two sites)
- Seen by: structure-prose [21], api-economics [47]; refutation: confirmed; history: already-known (R4 of the CBOR-wire review ruled the two-byte assumption wrong for epoch >= 24 and fixed tests/observe.rs; its site list omitted session_stats.rs)
- Owner-gated: no

The doc presents the label as a fixed two bytes, but the label is two CBOR unsigned-int heads, so an epoch of 24 or more encodes in two bytes and the label becomes three. The test runs one session on a fresh link (epoch 0), so it passes; the constant encodes a fixture fact as a protocol fact, and `rumors::testing::stream_label` already returns the parsed length. A quantity computable two ways gets computed one way or compared; here one suite computes and the other transcribes.

Evidence:

    246	/// Bytes of the label a sender writes before its first frame; the codec
    247	/// seam's counters exclude it, so the transport tally exceeds
    248	/// `bytes_sent` by exactly this much per opened stream.
    249	const LABEL_LEN: usize = 2;

    (src/tree/mirror/streaming/remote/streams.rs)
    64	    let mut label = Vec::with_capacity(cbor::head_len(u64::from(epoch)) + 1);
    65	    cbor::write_head(&mut label, MAJOR_UINT, u64::from(epoch));
    66	    cbor::write_head(&mut label, MAJOR_UINT, u64::from(stream.index()));

Resolution: Have `CountingWrite` retain each stream's first write and subtract `stream_label(first_bytes).1` per opened stream, or drive the test through `capture_sides` (which yields per-stream blobs) and sum `stream_label(blob).1`; delete `LABEL_LEN` and reword the doc to say the label length is parsed. Acceptance: no literal label length remains in session_stats.rs; the byte-tally assertion derives label bytes from `stream_label`.

### tests-observation-25: The conservation property never sheds: `messages_shed` is identically zero across the whole property
- Where: tests/session_stats.rs:297-335 (related: tests/session_stats.rs:91-116, tests/session_stats.rs:1-10, src/tree/mirror/streaming/stats.rs:82-98, src/tree/mirror/streaming/materialized/unknown.rs:94, src/tree/mirror/streaming/materialized/work/answer.rs:142, tests/common/action.rs:37)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the strategy: two `vec(any::<u64>())` draws, no redaction; read `honored_redaction_counts_as_shed`, the suite's only nonzero-shed witness; `grep '\.shed('` lists the five count sites in unknown.rs and answer.rs)
- Seen by: blind-spots [25]; refutation: confirmed, severity down to low (each shed count site has a committed deterministic point witness, at the public tier or the walk tier); history: no rationale found (sends-only since 7626543e4)
- Owner-gated: no

`sessions_conserve_the_live_count` draws only `a_sends` and `b_sends`; no case redacts, so `messages_shed` is 0 in every generated session and the law the module doc advertises as this suite's contribution ("`len_after = len_before + gained - shed` over real sessions") is exercised with its shed term structurally absent. The only public-surface shed witness is one point test with one side shedding one message. A counter that miscounts under a family-specific shape (both sides shedding, sheds inside disputed scopes, pruned-subtree sheds adding `node.len()`) passes this property. Doctrine: a strategy must reach the interesting region of the law it states. I keep this at medium against the refutation's low because the property is the suite's only family-level statement of the law and the change is small; the point witnesses do evidence the counter's liveness.

Evidence:

    302	    fn sessions_conserve_the_live_count(
    303	        a_sends in proptest::collection::vec(any::<u64>(), 0..24),
    304	        b_sends in proptest::collection::vec(any::<u64>(), 0..24),
    305	    ) {
    ...
    322	            prop_assert_eq!(
    323	                a.snapshot().len() as u64,
    324	                a_before + a_g.stats.messages_gained - a_g.stats.messages_shed,
    325	            );

Resolution: Build both sides on a shared converged base and let each side act: a preamble of shared sends before the fork, then `a = build_local(bootstrap_fork(&seed), &a_actions)` and `b = build_local(bootstrap_fork(&seed), &b_actions)` with `arb_local_actions()` (which draws `Redact`), so cross-redactions of base messages held on the other side occur. Keep the conservation and byte-mirror assertions; add a point case with both sides shedding. Acceptance: over a default run some cases report `messages_shed > 0` on each side (pin with a dedicated point case); a mutant deleting the `stats.shed(1)` call at answer.rs:142 fails this suite.

### tests-observation-26: stale_floor's testdoc quantifies over a family ("no matter how far") that the body samples at one point
- Where: tests/stale_floor.rs:27-31 (related: tests/stale_floor.rs:36, tests/stale_floor.rs:1-20, tests/bootstrap.rs:51-88)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read 27-37: the body sends exactly `0..16u64`)
- Seen by: structure-prose [20], blind-spots [32]; refutation: confirmed; history: already-known for the file's existence (Finch ruled on 2026-07-23 to re-denominate the module doc and keep the pin, 706352a3); the quantifier wording arrived at 1d3a3df4d with no note tying it to the count
- Owner-gated: no for the doc fix; deleting the file reopens the recorded ruling and goes to open questions

The doc claims the message survives regardless of how far the provider ticked; the body ticks exactly sixteen times. A point regression pin is sanctioned, but its doc then describes the point. The family version of this observable is already a property in tests/bootstrap.rs (`bootstrap_reproduces_a_fork`, 82-87, over `arb_local_actions()` provider histories, naming the same hazard), which predates the 2026-07-23 ruling.

Evidence:

    27	/// A message sent by a freshly-bootstrapped peer survives gossip, no
    28	/// matter how far the provider had ticked before serving the bootstrap:
    29	/// the served floor and the forked party region are paired atomically.
    ...
    36	            f.send_all(0..16u64).unwrap();

Resolution: Reword to the point pinned ("after the provider has ticked well past genesis") or lift `16` to `ticks in 1u64..64` under `proptest!`. Acceptance: the doc's quantifier matches the body's coverage.

### tests-observation-27: Ghost reference: the shadow-validity doc names an `on_message` callback the harness does not have
- Where: tests/shadow_validity.rs:15-17 (related: tests/partition.rs:12, tests/common/peer.rs:65-75)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn on_message tests/ src/` matches only the two doc comments; tests/common/peer.rs records observations via `drain` over `snapshot.range(causally::since(&self.checkpoint))`)
- Seen by: blind-spots [34]; refutation: confirmed; history: deliberate-but-expired (`on_message` was the callback of the original listen API, retired into the pull-based observers in db32b94d9 on 2026-06-10; both docs are older and were never re-denominated)
- Owner-gated: no

The module doc describes `observed_log` as predicting what each peer's `on_message` callback would fire; no such callback exists anywhere in tests/ or src/. The harness records observations by draining `Snapshot::range` into `Peer::observations`. Hard rule: no prose names code that does not exist.

Evidence:

    15	//! * `observed_log` — the set of `EventIdx`s the shadow predicts
    16	//!   each peer's `on_message` callback would have fired for must
    17	//!   match the set the live executor actually fired (for a retired

Resolution: Rewrite as "the set of `EventIdx`s the shadow predicts each peer's drain would record must match the set the executor's `observations` log holds"; fix tests/partition.rs:12 in the same pass. Acceptance: `grep -rn on_message tests/ src/` returns nothing.

### tests-observation-28: The overlap generator's shadow has no validity meta-test; the executor degrades a shadow mismatch to a silent skip; the shadow snapshots at `Open` while the real session forks after the preamble
- Where: tests/shadow_validity.rs:44-53 (related: tests/common/overlap.rs:228-242, tests/common/overlap.rs:178-180, tests/common/overlap.rs:92-117, tests/common/overlap.rs:636, tests/session_overlap.rs:155-158, src/peer/gossip.rs:625-629, src/peer/gossip.rs:695)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: assessed (read: `with_shadow` strategies exist only in tests/common/schedule/arb.rs; the overlap executor guards on `observed` with a comment saying a shadow imprecision degrades to a skipped event; `open()` builds futures without polling; the real session awaits `handshake::preamble` before `prior_tree = Some(inner.tree.clone())`; the shadow snapshots `sim.clone()` at `Open`)
- Seen by: api-economics [39]; refutation: confirmed with one correction (the shadow can also over-approximate: a `Redact` at `a` between `Open` and the real fork is live in the shadow's snapshot but absent from the wire fork, so "assert instead of guard" is safe only after the fork point is fixed); history: no rationale found for the missing twin or the inaccurate `Open` doc; the skip is a stated design choice in code
- Owner-gated: no

The serial schedule generator's shadow has a committed meta-test in this file for both alphabets. The overlap generator's `Knowledge` shadow, which governs which `Redact` events are emitted, has no `_with_shadow` strategy and no twin here, and the overlap executor turns a mismatch into a skipped event on both sides of the comparison, so shadow drift thins the tested redaction space with no failing signal. The shadow already disagrees with the code it models: it snapshots at `Open`, but a real session forks its working state only after the preamble exchange, so events between `Open` and the first `Step` ride the real session and not the modeled one, and the `Open` variant's doc ("forking both sides' working state here") is inaccurate against that path. Doctrine: a test blind spot is a finding; every hole becomes a committed check.

Evidence:

    44	proptest! {
    45	    /// For every peer, the shadow simulator's `observed_log` and
    46	    /// `live` sets (as `BTreeSet<EventIdx>`) match the live
    47	    /// executor's observations and current readout, translated
    48	    /// through `resolved_versions` back to event indices.
    49	    #[test]
    50	    fn shadow_predicts_live_state(
    51	        (schedule, shadow) in arb_schedule_with_shadow(any::<u64>(), N_PEERS, MAX_EVENTS),

    (tests/common/overlap.rs)
    233	                // The generator's shadow makes this always-observed; the
    234	                // guard mirrors the serial executor's, so a shadow
    235	                // imprecision degrades to a skipped event on both sides
    236	                // of the comparison rather than an invalid `redact`.
    237	                let observed = peers[*peer].observations.iter().any(|(v, _)| v == version);
    238	                if observed {
    239	                    peers[*peer].redact_one(version);
    240	                    oracle.redact(*target_event_idx);
    241	                }

Resolution: (1) Expose `arb_overlap_schedule_with_shadow` returning the final `Knowledge` and add the overlap twin of `shadow_predicts_live_state` to this file. (2) Decide, with that test in hand, whether the shadow should snapshot at the first `Step` (the real fork point) rather than at `Open`, and fix the `Open` doc to say when the fork happens. (3) Only after the fork point matches the code, change the executor's guard to an assertion so a shadow mismatch fails instead of skipping. Acceptance: a committed test compares the overlap shadow's per-peer `observed_log`/`live` to the executor's; the executor asserts rather than skips; the `Open` doc states the fork point accurately.
Construction: Change `Knowledge::merge_session` in tests/common/overlap.rs to deliver nothing for a `Close`; today every property in session_overlap.rs still passes because the affected `Redact`s are never generated or are skipped. With the meta-test in place it fails.

### tests-observation-29: `execute_with(..., |_, _, _| true)` where `execute` already spells that
- Where: tests/shadow_validity.rs:54 (related: tests/common/schedule/executor.rs:66-71, tests/shadow_validity.rs:35-38)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read executor.rs:66-71)
- Seen by: structure-prose [16]; refutation: confirmed; history: no rationale found (both born in 8b1ade010)
- Owner-gated: no

`execute(schedule, windows)` is defined as exactly `execute_with(schedule, windows, |_, _, _| true)`; the call here spells the long form and imports only `execute_with`.

Evidence:

    54	        let result = execute_with(&schedule, &windows, |_, _, _| true);

    (tests/common/schedule/executor.rs)
    66	pub fn execute<T>(schedule: &Schedule<T>, windows: &WindowAssignment) -> ExecutionResult<T>
    ...
    70	    execute_with(schedule, windows, |_, _, _| true)

Resolution: `let result = execute(&schedule, &windows);` and import `execute` instead of `execute_with`. Acceptance: `grep -n execute_with tests/shadow_validity.rs` is empty.

### tests-observation-30: The two shadow-validity tests duplicate the version-to-event translation
- Where: tests/shadow_validity.rs:55-72 (related: tests/shadow_validity.rs:101-105, tests/shadow_validity.rs:127-136)
- Class / severity / confidence: simplification / nit / medium
- Provenance: assessed (read both tests)
- Seen by: structure-prose [23]; refutation: confirmed; history: no rationale found
- Owner-gated: no

Building `version_to_event_idx` (55-59 and 101-105) and translating an observation list into a `BTreeSet<EventIdx>` for comparison with `shadow.observed_log[p]` (62-72 and 127-136) are repeated between the two tests; two copies are two places for the `version_key` discipline to drift. A helper leaves each body as its membership-specific logic.

Evidence:

    55	        let version_to_event_idx: BTreeMap<Vec<u8>, EventIdx> = result
    56	            .resolved_versions
    57	            .iter()
    58	            .map(|(eid, v)| (version_key(v), *eid))
    59	            .collect();

Resolution: Add `fn event_index_map(resolved: &BTreeMap<EventIdx, Version>) -> BTreeMap<Vec<u8>, EventIdx>` and `fn observed_events<'a>(observations: impl Iterator<Item = &'a Version>, map: &BTreeMap<Vec<u8>, EventIdx>) -> BTreeSet<EventIdx>` at file scope and use them in both tests (the overlap twin from tests-observation-28 would be a third user). Acceptance: each translation appears once in the file.

### tests-observation-31: "The Law of Disjointness" is a capitalized coinage with no anchor in `before`
- Where: tests/party_conservation.rs:16-18 (related: tests/party_conservation.rs:66, tests/party_conservation.rs:200, crates/before/src/laws.rs:2033)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn 'Law of Disjointness' src tests crates/before/src` matches only these three lines; `before`'s laws module names the property `fork_halves_disjoint`)
- Seen by: structure-prose [14]; refutation: confirmed; history: deliberate-but-expired (the term was `before`'s own in its party.rs module doc when this suite adopted it; b3f09baa0 de-capitalized it to plain "disjointness", orphaning these three uses)
- Owner-gated: no

The phrase appears three times here as a proper noun and nowhere else in the repository. A reader looks for the Law and finds nothing to cite. Established terms of art only; if a `before` law is meant, cite it by name.

Evidence:

    16	//! 1. **Disjointness** (the Law of Disjointness): all live parties are
    17	//!    pairwise disjoint after every step of an arbitrary lifecycle
    18	//!    schedule.

Resolution: Replace "the Law of Disjointness" with "party disjointness" (or cite `before`'s `fork_halves_disjoint` law by name where the kernel-checked statement is meant) at 16, 66, 200. Acceptance: `grep -rn 'Law of Disjointness' tests/` is empty.

### tests-observation-32: Six suites redefine a 64 KiB `LINK_BUF` for the retire path with a rationale the harness falsifies, listen.rs inlines the literal, and four suites reimplement the retire driver
- Where: tests/party_conservation.rs:48-51 (related: tests/party_conservation.rs:103-120, tests/listen.rs:252-265, tests/listen.rs:259, tests/retire.rs:31-33, tests/retire.rs:52-65, tests/bootstrap.rs:27, tests/gossip_when.rs:57, tests/reuse.rs:31, tests/bookmark_attach.rs:20, tests/common/wire.rs:61-63, tests/common/schedule/executor.rs:20, tests/common/schedule/executor.rs:256-275, tests/common/overlap.rs:57, tests/bookmark_causality.rs:80, tests/bookmark_causality.rs:673)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`grep -rn '64 \* 1024\|const LINK_BUF' tests/` lists the six per-suite constants, the bare literal at listen.rs:259, and `wire::LINK_BUF = 8 * 1024`; read the four driver bodies; executor.rs imports `LINK_BUF` from `crate::common::wire` and retires over it at 258)
- Seen by: structure-prose [8], blind-spots [36], api-economics [42]; refutation: confirmed (42's severity down to low: harness duplication without correctness exposure; the constant confusion is the substantive part); history: deliberate-but-expired (retire.rs's "other wire tests' headroom" dates from when every wire test used a 64 KiB duplex; b3b877d9b created the shared 8 KiB constant and every private 64 KiB one in a single commit, so the premise expired in the commit that wrote it; party_conservation.rs then copied retire.rs's sentence)
- Owner-gated: no

`common::wire::LINK_BUF` is 8 KiB. Six suites define a private `LINK_BUF = 64 * 1024` for retire and bootstrap sessions, each justified by pointing at another file (here "keep `retire.rs`'s headroom"; retire.rs says "keep the other wire tests' headroom", which the shared constant contradicts), and listen.rs inlines `64 * 1024` with no name. Meanwhile tests/common/schedule/executor.rs runs every scheduled retirement over the shared 8 KiB constant, tests/bookmark_causality.rs retires over its own 8 KiB, and tests/common/overlap.rs runs whole sessions at 48 bytes; the link contract promises receiver-paced flow control at any positive capacity, so the 64 KiB "headroom" is not a requirement. Two constants named `LINK_BUF` with different values across one suite mislead a reader who knows the shared one. The retire driver itself (`try_into_peer`, `memory_with_capacity`, `join!(retire, gossip)`, expect the absorber, `assert_control_drained`, match `Retire::Retired`) is spelled four times, and they have already drifted: executor.rs:266-273 matches every `Retire` variant with a distinct panic while the other three use `matches!`/`assert!`.

Evidence:

    48	/// Capacity for each in-memory link stream on the retirement path: a
    49	/// divergent retiree's session moves content through its gossip round, so
    50	/// keep `retire.rs`'s headroom.
    51	const LINK_BUF: usize = 64 * 1024;

    (tests/retire.rs)
    31	/// Capacity for each in-memory link stream. A divergent retiree's session moves
    32	/// content through the gossip round, so keep the other wire tests' headroom.
    33	const LINK_BUF: usize = 64 * 1024;

    (tests/common/wire.rs)
    63	pub const LINK_BUF: usize = 8 * 1024;

    (tests/common/schedule/executor.rs)
    258	                let (mut link_r, mut link_a) = rumors::link::memory_with_capacity(LINK_BUF);

Resolution: Add `pub fn retire_into<T>(retiree: Rumors<T>, absorber: &Rumors<T>) -> Retire<T>` (with an `_async` core taking the `Peer`) to `tests/common/wire.rs`, using `wire::LINK_BUF` and the executor's per-variant panics as the diagnostic; migrate the four drivers and the executor arm; delete the six per-suite constants where the retire helper was their only use (bootstrap.rs, gossip_when.rs, reuse.rs, bookmark_attach.rs each need a look at their other uses). If a run shows some retire path needs more than 8 KiB, add one `RETIRE_LINK_BUF` to `common::wire` whose doc states the actual reason. Also sweep the three private `LINK_BUF = 8 * 1024` copies (bookmark_when.rs:69, bookmark_causality.rs:80, bookmark_transmit_window.rs:49) onto the shared constant. Acceptance: `grep -rn 'const LINK_BUF\|64 \* 1024' tests/*.rs` shows no per-suite `LINK_BUF` and no inline capacity literal at listen.rs:259; one retire driver under tests/common.

### tests-observation-33: Em-dashes in `//` comments and in one assert message
- Where: tests/party_conservation.rs:91-95 (related: tests/causal.rs:144-145, tests/causal.rs:555-556, tests/causal.rs:655-656, tests/listen.rs:497, tests/listen.rs:517, tests/listen.rs:634)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -n '^\s*//[^/!].*—'` over the partition lists the seven comment sites; the assert message at 93-94 read directly)
- Seen by: structure-prose [15]; refutation: confirmed (tree-wide: 116 `//` lines in src/, 43 in tests/); history: already-known (R76 of the link-transport review recorded the assert-message rule and was applied at one site; no tree-wide sweep followed)
- Owner-gated: no

The doctrine wants spaced double-hyphens in code comments and colons or double-hyphens in log and assert text (terminal compatibility), reserving em-dashes for rendered prose. Seven `//` comments in the partition and one `assert!` message use `—`. The fix is a mechanical sweep, tree-wide; this partition's sites are listed.

Evidence:

    91	    assert!(
    92	        whole == Party::seed(),
    93	        "the join of all live parties must be invariant — exactly the seed's \
    94	         whole interval — after every step; got {whole:?}"
    95	    );

Resolution: Replace `—` with ` -- ` or a colon at party_conservation.rs:93-94, causal.rs:144-145, 555-556, 655-656, listen.rs:497, 517, 634; consider the same sweep tree-wide in one commit. Acceptance: `grep -rn '^\s*//[^/!].*—' tests/{causal,listen,party_conservation}.rs` is empty and the assert message at 93-94 has no em-dash.

### tests-observation-34: `assert!(a == b, "... {a:?} vs {b:?}")` where the equality macros print both sides
- Where: tests/party_conservation.rs:285-290 (related: tests/party_conservation.rs:91-95, tests/party_conservation.rs:257-261, tests/party_conservation.rs:326-330, tests/party_conservation.rs:372-376, tests/party_conservation.rs:436-440)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read the six sites; verified proptest 1.11.0's `prop_assert_eq!` binds `let left = $left; let right = $right;` (sugar.rs:793-795), moving its operands, and `Party` is `!Clone` by `assert_not_impl_any!` at party.rs:74)
- Seen by: structure-prose [18]; refutation: confirmed with the `&` caveat; history: no rationale found
- Owner-gated: no

Six equality assertions are written as `assert!`/`prop_assert!` on `==` with a hand-formatted message reproducing the operands; `assert_eq!`/`prop_assert_eq!` print both sides and read as an equality at a glance. The file already uses `prop_assert_eq!` at 331 and 377. Because `prop_assert_eq!` moves its operands and `Party` is not `Clone`, the loop site at 326-330 must pass `&now, &baseline`.

Evidence:

    285	        let absorber_post = alias(&absorber);
    286	        prop_assert!(
    287	            absorber_post == expected,
    288	            "the absorber's post-party must be absorber-pre ⊔ retiree-pre: \
    289	             {absorber_post:?} vs {expected:?}"
    290	        );

Resolution: `prop_assert_eq!(absorber_post, expected, "...")` here and at 257-261, 372-376, 436-440; `prop_assert_eq!(&now, &baseline, "...")` at 326-330; `assert_eq!(whole, Party::seed(), "...")` at 91-95; drop the `{x:?} vs {y:?}` tails. Acceptance: no `assert!(... == ...)` or `prop_assert!(... == ...)` remains in the file.

### tests-observation-35: The `encoded_bits` assertions are implied by the preceding `Party` equality and are the sole reason the `meter` dev-feature is enabled
- Where: tests/party_conservation.rs:326-335 (related: tests/party_conservation.rs:314, tests/party_conservation.rs:377-381, tests/party_conservation.rs:29-32, Cargo.toml:146-149, crates/before/src/party.rs:82-85, crates/before/src/party.rs:597-600)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`Party: PartialEq` is `codec::canonical_eq`, a raw-byte compare under marker padding that pins the live length inside the final byte; `encoded_bits` is `self.0.len()`; `grep -rn encoded_bits src tests benches examples Cargo.toml` finds only this file and the Cargo.toml comment; rumors' own `meter` feature routes to `before/limb-meter` and `before/scan-meter`, which imply `meter`, so src/tree/tests.rs's `before::meter` use does not depend on the dev-dependency's feature)
- Seen by: structure-prose [7]; refutation: confirmed; history: deliberate-and-holds (6fef7eb3 wrote the size clause as the fragmentation bound's own denomination; 05d87e1b records Finch's direct ruling routing `encoded_bits` behind `meter` and lighting it in rumors' dev-dependency "for the conservation suite, whose invariant is unchanged"; that ruling did not consider the redundancy)
- Owner-gated: yes: reopens the 05d87e1b routing with new evidence (the implication)

Under `Party`'s byte-level equality, `now == baseline` implies `now.encoded_bits() == baseline.encoded_bits()`, so the two `encoded_bits` assertions recompute a deterministic O(1) function of values just asserted identical; recompute-and-compare on a pure function is not defense in depth. They are also the only consumer of the dev-dependency's `meter` feature (Cargo.toml:146-149 says so). The counter-argument the history pass makes is fair: the size assertion states the bound independently of how `PartialEq` happens to be implemented, and `Party`'s doc commits to byte-level equality by design (party.rs:76-81). Whether that independence is worth a dependency feature is the owner's call.

Evidence:

    326	            prop_assert!(
    327	                now == baseline,
    328	                "cycle {cycle}: the provider's party must return to its \
    329	                 baseline, got {now:?} vs {baseline:?}"
    330	            );
    331	            prop_assert_eq!(
    332	                now.encoded_bits(), baseline_bits,
    333	                "cycle {}: the party's encoded size must not grow with the \
    334	                 cycle count", cycle
    335	            );

    (crates/before/src/party.rs)
    82	impl PartialEq for Party {
    83	    fn eq(&self, other: &Self) -> bool {
    84	        codec::canonical_eq(&self.0, &other.0)
    85	    }
    ...
    597	    pub fn encoded_bits(&self) -> u64 {
    598	        // The stored form's O(1) length: exact at every size memory holds.
    599	        self.0.len()
    600	    }

Resolution: If the owner agrees: delete the `encoded_bits` assertions and `baseline_bits` (314, 331-335, 377-381); reword module-doc item 4 (29-32) to state the size bound as a consequence of bit-for-bit return ("so the encoded size cannot grow with churn"); drop `"meter"` from the `before` dev-dependency and its three-line comment in Cargo.toml. If the owner keeps them: add one sentence at 331 stating that the size check is kept as an implementation-independent statement of the bound. Acceptance: either `grep -rn encoded_bits src tests benches examples Cargo.toml` is empty and `just gate` passes with `before = { workspace = true, features = ["serde"] }` under `[dev-dependencies]`, or the retained assertion carries its rationale at the site.

### tests-observation-36: The fleet-scale retirement test constructs content movement and asserts only identity
- Where: tests/party_conservation.rs:402-441 (related: tests/party_conservation.rs:409-413, tests/common/oracle.rs:91-100, tests/retire.rs:67-70)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read 414-441: the only `prop_assert!` is on `survivor == Party::seed()`; each newcomer sends `i as u64` once)
- Seen by: blind-spots [27]; refutation: confirmed; history: deliberate-and-holds for the file's charter (identity invariants, line 1; 574559635 asserts exactly the survivor's party), with the comment at 409-413 reading as a claim the test does not check
- Owner-gated: no

The test grows a fleet of up to a hundred peers, has each newcomer originate one unique value "so retirements move content, not just identity", retires everyone into one survivor, and asserts only that the survivor's party is `Party::seed()`. Nothing checks that the survivor holds the originated messages, so a retirement path that moves identity but drops content passes here; content-moving retirement is asserted elsewhere only at small scale (retire.rs's `retire_into_counting_gossip`). The comment states an observable the test does not assert; either assert it (cheap) or reword the comment to say the sends exist to exercise the retire path under content.

Evidence:

    409	        // Grow the fleet one bootstrap at a time through `apply`, which
    410	        // resolves provider indices modulo the live fleet: any random
    411	        // topology (chain, fan, or mixture) is reachable. Each newcomer
    412	        // originates once (`Bootstrap` pushes it at the fleet's end), so
    413	        // retirements move content, not just identity.
    ...
    435	        let survivor = alias(&fleet[0]);
    436	        prop_assert!(
    437	            survivor == Party::seed(),

Resolution: After the retirement loop, `prop_assert_eq!(readout_multiset(&fleet[0].snapshot()), (0..providers.len() as u64).map(|i| (i, 1)).collect::<BTreeMap<_, _>>())` (importing `readout_multiset` from `crate::common::oracle`), or at least `prop_assert_eq!(fleet[0].snapshot().len(), providers.len())`. Acceptance: a mutant that skips content reconciliation in the retire path fails this test.
Construction: Temporarily make the retiree's write-back in `retire_inner` skip the gossip round's content while still donating the party; today this test passes; with the readout assertion it fails.

### tests-observation-37: The gate's `testdoc` regex does not recognize `#[pollster::test]`, leaving seven changes.rs tests (and two other suites) outside doc-comment enforcement
- Where: tools/testdoc:20-23 (related: tests/changes.rs:18, 29, 58, 73, 103, 142, 158, tests/handshake.rs, tests/gossip_when.rs, justfile:184-186)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (ran the regex in python3: `#[pollster::test]` does not match while `#[test]` and `#[tokio::test(flavor = "current_thread")]` do; ran `./tools/testdoc` on scratch copies: tests/changes.rs with the doc above line 18 removed exits 0, tests/listen.rs with the doc above line 76 removed exits 1 naming `genesis_replay_observes_the_live_set_once`; `grep -c pollster::test` gives changes.rs 7, handshake.rs 7, gossip_when.rs 2)
- Seen by: api-economics [40]; refutation: confirmed by the same construction; history: no rationale found (the tool predates pollster's appearance in tests/ by one day; the hole also covers the src/ pollster suites since the recipe scans the whole tree)
- Owner-gated: no

`tools/testdoc` recognizes `test`, `tokio::test`, `async_std::test`, `rstest`, and `test_case`, but not `pollster::test`. AGENTS.md says the gate's `testdoc` checks that every test's doc comment exists; a check that skips one attribute in use is a gate with a hole, and the tests happen to be documented today, which is exactly the condition under which the hole is invisible.

Evidence:

    20	TEST_ATTRIBUTE = re.compile(
    21	    r"^\s*#\[\s*(?:test|tokio::test|async_std::test|rstest|test_case)"
    22	    r"\s*(?:\([^]]*\))?\s*\]\s*$"
    23	)

Resolution: Add `pollster::test` to the alternation (or match any `<path>::test` form) and add a `#[pollster::test]` case to `--self-test`. Independently, tests-observation-8 removes pollster from changes.rs. Acceptance: `./tools/testdoc --self-test` covers `#[pollster::test]`; stripping the doc above tests/changes.rs:18 fails `just testdoc`.
Construction: Copy tests/changes.rs to a scratch path, delete lines 16-17, run `./tools/testdoc <copy>`: exit 0 today.

### tests-observation-38: Seed files carry a parked "awaits owner disposition" note and past-tense "minted" provenance comments
- Where: proptest-regressions/shadow_validity.txt:9-14 (related: proptest-regressions/session_stats.txt:7, proptest-regressions/tree/mirror/streaming/tests/stats.txt:7, proptest-regressions/tree/mirror/streaming/tests/faults.txt:11, proptest-regressions/tree/mirror/streaming/tests/faults.txt:13, proptest-regressions/tree/mirror/streaming/materialized/work/tests/violations.txt:7, proptest-regressions/tree/mirror/streaming/remote/codec/decode/tests.txt:7)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both partition seed files; `grep -rn minted proptest-regressions/` lists six files; `git log -S'awaits owner disposition'` gives 390160a76 on 2026-08-13; `git show -s 2c73d032` records the "mint" purge's carve-out)
- Seen by: blind-spots [33] [35], api-economics [49]; refutation: confirmed, with two corrections carried here (the disposition note is from 390160a76, not ce3664dd, which moved the file untouched; 2c73d032 deliberately exempted the session_stats comment as "a committed seed artifact that stays byte-stable"); history: already-known on both halves (a ruling pending since 2026-08-13; a ruling that undercounted six occurrences as one and whose byte-stability premise concerns stripping seeds, which editing a `#` comment does not do)
- Owner-gated: yes: both halves are recorded rulings

The third `cc` line in shadow_validity.txt carries a note saying it predates the current `Schedule` shape, "no longer replays" the failure it was written for, and "awaits owner disposition". Proptest reads only the hash, so the line still seeds one deterministic case and is not dead; but the tree holds a dated, undisposed note, and "awaiting" is not a state the tree may hold: every contradiction resolves to a fix or a declared model. The session_stats seed's comment explains the seed's origin in past tense with "minted", vocabulary the crate purged; the purge exempted it on a premise (seed byte-stability) that AGENTS.md attaches to `cc` lines, not comments, and five sibling seed files carry the same pattern.

Evidence:

    9	# The next entry predates the current Schedule strategy shape (its shrink
    10	# note lacks fork_parents), so it no longer replays the failure it was
    11	# written for. It awaits owner disposition; do not strip it without a
    12	# ruling. Proptest reads only the hash before the first '#' on a cc line,
    13	# so this comment and the stale shrink note cost nothing at replay.

    (proptest-regressions/session_stats.txt)
    7	# The case below was minted against a deliberately unwired recorder during red-first verification; it passes on real code.

Resolution: Rule now. Recommended: keep the `cc 86723fa8...` hash as a valid deterministic replay case under the current strategy, delete its stale shrink note after the `#` and the five-line comment, recording the ruling in the commit message; for the six "minted" comments, either delete them (git has the provenance) or restate each in the present tense ("# A deterministic minimal replay case; passes on the current code."). Acceptance: no comment in any seed file speaks of pending disposition; `grep -rn minted proptest-regressions/` is empty; `tests/seed_liveness.rs` stays green.

## Positives

- Every wire session in the partition that owns both link ends finishes with `assert_control_drained` (listen.rs:263, party_conservation.rs:113, session_stats.rs:39, and every `common::wire` driver), turning a latent control-stream desynchronization into a failure at the session that caused it.
- Every session runs under `common::wire::block_on`, which is `run_to_quiescence(..).expect(..)`: a wire stall fails at the stalled poll with a verdict rather than hanging (changes.rs is the one exception, tests-observation-8).
- session_stats.rs's `CountingWrite`/`CountingConnector` (161-211) tally at the transport layer, so `bytes_sent` is checked against a number the crate's own counters cannot influence; the label exclusion is accounted for explicitly; the inline `Link<...>` return type with `#[allow(clippy::type_complexity)]` (215-227) follows the doctrine of not coining a synonym to appease the lint.
- listen.rs's `folding_delivered_versions_can_lose_a_message` (464-526) is a committed negative control: it searches deterministic universes for the path-before-version shape, shows the fold loses a message, and shows `checkpoint()` re-delivers it. That is the argument for the API shape, built rather than stated.
- causal.rs's `restart_replays_every_unhandled_message` (536-633) and `final_pop_checkpoint_still_replays_the_last_message` (651-682) model the persist-after-delivery crash protocol on both observer faces, round-tripping the checkpoint through `as_bytes`/`Version::decode` and resuming against a replica rebuilt from a survivor: the realistic restart shape.
- changes.rs's `gossip_frontier_only_advance_ticks_the_observer` (87-137) names the contract clause, the mechanism that makes the case special (`Tree::join`'s changed flag excludes ceiling-only advances), and why `now_or_never() == None` there would be a lost wakeup rather than a pending one.
- observe.rs holds the hook's view against an independent transport capture byte for byte in both directions and separately proves observed-versus-unobserved wire identity over whole universes (366-403), explaining at the fixture (259-260) why the network id must be drawn from a fixed seed.
- session_overlap.rs's sweep is total: it calibrates the poll ceiling on the same fleet shape (81-93), sweeps every redaction target and every parking prefix, and collects violations instead of stopping at the first, so a regression reports its whole footprint. tests/common/overlap.rs:46-57 states why a 48-byte link buffer is the point of the harness.
- party_conservation.rs states four invariant families with the model each rests on (12-37), checks disjointness pairwise so a violation names the pair (66-67), gets the index arithmetic after `fleet.remove(r)` right (178-180), and places the reduced-case-count rationale as a comment on the `proptest_config` it constrains (386-392).
- shadow_validity.rs exists at all: a meta-test that the generator's validity guarantee holds against the executor for both alphabets, compared set-wise with the reason stated (25-26).
- Every proptest in the partition whose per-case cost is a wire session states why its case count is pared (observe 24, session_overlap 48, session_stats 24, fleet-scale 32).

## Open questions for Finch

- Causal delivery order (tests-observation-3): promote the replica-independent `(rank, canonical bytes)` order into the public `CausalMessages` contract, or keep the public under-promise and re-label the two tests as internal-order pins? Recommendation: promote. The order is a function of the set alone, the field doc already relies on it, and a user building a replicated log on `CausalMessages` will want to rely on it too; the materialized-backlogs note treats global rank order as load-bearing.
- `encoded_bits` and the `meter` dev-feature (tests-observation-35): drop the redundant assertions and the feature, or keep the size check as an implementation-independent statement of the fragmentation bound? Recommendation: drop; `Party`'s doc commits to byte-level equality by design, and a dependency feature whose only consumer is an implied assertion is the circular-justification tell.
- Seed comments (tests-observation-38): keep the third shadow_validity `cc` line (with its stale note removed) or strip it; delete or reword the six "minted" comments? Recommendation: keep the hash, delete every comment; provenance lives in git.
- `converged_trio` (tests-observation-22): was the default window on `a` needed to reproduce the discovering incident, or is it an omission? Recommendation: if the incident reproduced only at the default window, say so in the doc and keep it; otherwise pin the floor for uniformity.
- tests/stale_floor.rs: the same observable is a committed property in tests/bootstrap.rs (`bootstrap_reproduces_a_fork`, 82-87), which predates your 2026-07-23 ruling to keep the pin. Fold the newcomer-side check (`b_has`) into that property and delete the file, or keep the named pin with its doc fixed (tests-observation-26)? Recommendation: fold and delete, since the property already asserts the provider side over arbitrary histories; but this reopens a ruling and I have no new evidence beyond the duplication you presumably knew of.
- Does any retire path actually need a 64 KiB link (tests-observation-32)? The schedule executor and bookmark_causality.rs retire at 8 KiB by reading, and the link contract promises flow control at any positive capacity. I could not run the suite to confirm; if a retire session stalls at 8 KiB under `run_to_quiescence`, that is a production finding, not a test-constant one.
- `Changes` yields on `latest()` changing (src/rumors/changes.rs:90-93, 147-150) while the gossip commit notifies on `peer_retiring || tree_changed || ceiling_advancing` (src/peer/gossip.rs:865-870). A content change with no frontier advance would be a lost tick. The blind-spots lens and I could not construct one (every gain brings a version outside the local frontier; every shed requires the peer's frontier to dominate the leaf, which the same join brings in). If you see a path, `Changes` under session overlap is untested; if not, one sentence in the `Changes` field doc stating why `latest()` suffices would close the reading.
- tests/common/peer.rs's `drain` reimplements the `UnorderedMessages` pass via `Snapshot::range(since(checkpoint))`; every schedule and overlap suite validates against this reimplementation, not the public observer. Is a one-property bridge wanted (drain a harness `Peer` and an `UnorderedMessages` on the same replica in lockstep and assert set-equal observations), so the public observer cannot drift from the harness's model of it unnoticed? Recommendation: yes, one property in listen.rs.
- Should the overlap shadow snapshot at the first `Step` (the real fork point, after the preamble exchange) rather than at `Open` (tests-observation-28)? Answering this properly wants the meta-test in place first.
- listen.rs:5-6 points readers to the `Snapshot::range` differential in src/tree/tests.rs, but the test there (`range_and_freeze_match_the_naive_filter`, 886) exercises `Tree::range`/`Tree::range_owned`, internal entries. Whether a public-surface differential for `Snapshot::range` exists is a question for the tree partition's reviewer; the pointer here should name whichever test covers the public method.
- With `Protocol` down to one variant, does `SessionInfo.protocol` stay on the hook surface? The V1-retirement note keeps it as wire vocabulary; this is a production API decision outside the partition, and tests-observation-13 follows either way.
- The total sweep in `overlapped_install_never_loses_innocent_messages` runs on the order of 25 × (session_polls + 1) fleets, each with two bootstraps, one 48-byte-buffer overlapped session, one gossip, and a convergence of at least three sessions; no committed wall-time figure exists for it, and nextest's 180 s terminate is the only guard. Worth one measured number in a note, not a code change.

## Dropped

- [26] retire_ends_the_observer doc (blind-spots): duplicate of tests-observation-17.
- [37] observer helpers (blind-spots) and [41] (api-economics): duplicates of tests-observation-2.
- [3] gossip_pair and [4] corpora (structure-prose): merged into tests-observation-12 with [45].
- [36] LINK_BUF (blind-spots) and [42] retire driver (api-economics): merged into tests-observation-32 with [8].
- [43] pollster (api-economics): merged into tests-observation-8 with [13].
- [46] § tags (api-economics): duplicate of tests-observation-15; its count of thirteen corrected to fourteen.
- [48] braces (api-economics): duplicate of tests-observation-16.
- [52] Protocol::V2 (api-economics) and [53] opened predicate: duplicates of tests-observation-13 and -10.
- [54] imports (api-economics): merged into tests-observation-21 with [17].
- [47] LABEL_LEN (api-economics): merged into tests-observation-24 with [21].
- [49] seed comments (api-economics), [33] and [35] (blind-spots): merged into tests-observation-38; [35]'s claim that the "mint" purge did not reach proptest-regressions/ is corrected (2c73d032 exempted the file deliberately); [33]'s attribution to ce3664dd is corrected to 390160a76.
- [51] converged_trio window (api-economics): duplicate of tests-observation-22 with [38].
- [24]'s sub-claim that a KV-backed backlog could invert rank within a pass: contradicted by the materialized-backlogs note ("rank-ordered by construction"); the finding survives without it as tests-observation-3.
- [0]'s sub-claim that the repeated §6.6 and §6.9 tags are ordering errors: the recovered plan carries a "Negative control" under item 6 and a "Variant" under item 9, so the doubles are faithful to the plan; the finding survives as orphaned tags (tests-observation-15).
- [32]'s resolution to delete tests/stale_floor.rs: reopens Finch's 2026-07-23 ruling, and the bootstrap.rs family pin it cites predates that ruling, so it brings no new evidence; moved to open questions, with the doc-quantifier defect kept as tests-observation-26.
- [39]'s step (a) "assert instead of guard" as an immediate change: refuted in part (the shadow can over-approximate before the fork point is fixed); retained as the conditional third step of tests-observation-28.
- [22]'s generation-time framing: not load-bearing (`Index::index(n)` also takes its bound at execution time); the shrinking argument survives as tests-observation-6.
- [6]/[52] owner-gating: dropping the vacuous assertion does not touch the `SessionInfo.protocol` field the retirement note keeps, so tests-observation-13 is not owner-gated.
- Refutation new item 3 (three private `LINK_BUF = 8 * 1024` copies in the bookmark suites): out of partition; listed as a related sweep in tests-observation-32's resolution.
- Refutation new item 4 (executor.rs's per-variant retire diagnostics): folded into tests-observation-32 as the diagnostic to keep.
- Refutation new item 5 (trailing commas inside `tokio::join!(..., )` at observe.rs:283, listen.rs:261, retire.rs:60): below the bar; rustfmt does not reflow macro bodies and the commas carry no cost.
- observe.txt seed shrinking to an all-empty universe (blind-spots open question): no comment, no defect; nothing to do unless the seed-comment ruling in tests-observation-38 adopts a uniform note.
