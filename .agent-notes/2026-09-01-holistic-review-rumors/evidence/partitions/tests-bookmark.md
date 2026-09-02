# Partition tests-bookmark: Integration tests: bookmark attach, causality, transmit window, when

## Partition summary

The four suites pin the identity bookmark, the crate's durable checkpoint of a peer's `Party` and `Version`, from four angles. `tests/bookmark_attach.rs` (210 lines) holds three point tests on `Peer::bookmark` after construction: a pristine seed writes nothing at attach, a failed persist hands the peer back for retry, and a failed attach never reclaims a stranded region into the returned peer. `tests/bookmark_causality.rs` (1357 lines) is a deterministic, plan-driven fleet simulation: a `World` of `Node`s over `FlakyInMemoryBookmark`, wire faults from `common::fault`, crashes and retirements, judged by an `EmissionLog` that rejects any later durable emission whose version is `<=` an earlier one, plus post-heal convergence, party disjointness, and (reliable variant) identity-space coverage. `tests/bookmark_transmit_window.rs` (654 lines) builds a `GatedBookmark` that parks inside the durable write so a concurrent `send` becomes a deterministic interleaving point, and pins that the persisted record dominates every own-party event a session transmits, including after a cancelled persist and across donation-persist aborts. `tests/bookmark_when.rs` (778 lines) instruments the `Bookmark` trait with a logging `Probe` and checks the read/write call schedule against an operation-semantics `Model` that deliberately shares none of the crate's suppression arithmetic. All 2999 lines are test code; I also read the harness modules they lean on (`tests/common/{wire,sim,flaky,fault,mod}.rs`) and the crate sources their claims cite.

The substance is strong. The properties are the right ones, the oracles are independent of the implementation where independence matters, the proptests carry negative controls and vacuity guards, the two committed seeds match their explicit reconstructions byte for byte, and every suite drives the public API plus the gated `test-internals` aliases only. `GatedBookmark` and the red-first history of the transmit-window pins are instruments-before-cures done exactly right. The `retire` step's refusal to swallow the absorber's result, with the decode-failure classification stated inline, is the standard the rest of the harness should be held to.

The dominant issues are two harness blind spots and a maintenance-shape problem. First, the causality suite's recycle oracle only sees the degenerate recycle (a version equal to or below an earlier one); the recycle the bookmark exists to prevent produces a version that compares `Greater` or incomparable, and since redactions are untracked the destroyed message is invisible too, so the proptest's headline claim is judged by an oracle blind to its primary failure class (the crate itself is covered by the transmit-window pin). Second, the same suite's `bootstrap_into` and `gossip` swallow session errors, including server-side panics, in regimes where any error is unconditionally a crate bug, and its "fully deterministic" claim is false because `Peer::seed()` draws `Network` ids from `OsRng` and the ids decide the tie-break. Third, `common::wire`'s drivers are fixed at `NoBookmark`, so every suite re-implements bootstrap-serving and pair-gossip (losing the drained-control assertion the shared drivers enforce), re-declares `LINK_BUF` and the heal cap, and repeats its preludes. The rest is prose: past-tense bug narrative, em-dashes in `//` comments, fragment lines left by the first-sentence split, and a few dialect tells.

## Findings

### tests-bookmark-1: Module doc counts "two corners" over three tests
- Where: tests/bookmark_attach.rs:4-8 (related: tests/bookmark_attach.rs:51, 78, 139-140)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (counted the `#[test]` functions at L51, L78, L139; `git log -S'two corners'` resolves to 624917eb, whose message names all three tests)
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (the count was incomplete at birth, not rotted)
- Owner-gated: no

The module doc names two corners of the attach contract, but the file holds three tests, and the third, `failed_attach_does_not_reclaim_into_an_unbookmarked_peer`, pins the reclaim-at-attach hazard that is neither named corner. A hand-maintained count in the reader's map of the file omits its most intricate test (Principle 5: no hand-maintained counts).

Evidence:

         4	//! The property suite in `bookmark_causality.rs` exercises the eager-persist
         5	//! and fault paths in aggregate; these are point assertions on the two corners
         6	//! of the attach contract that suite never names directly: that a pristine seed
         7	//! is persisted lazily (no write at attach time), and that a failed persist
         8	//! hands the peer back intact for a retry rather than stranding its identity.

Resolution: drop the count and name all three claims (lazy persist for a pristine seed; a failed persist returns the peer intact; a failed attach reclaims nothing into the returned peer), or state that these are point tests on the attach contract's corners and let the testdocs enumerate. Acceptance: the module doc's list matches the `#[test]` functions in the file.

### tests-bookmark-2: `LINK_BUF` and the heal-round cap are re-declared per suite, one value contradicting a sibling's "matching" claim
- Where: tests/bookmark_attach.rs:19-20 (related: tests/bookmark_causality.rs:78-80, 82-85, 760; tests/bookmark_transmit_window.rs:37, 47-53, 461-463; tests/bookmark_when.rs:68-69; tests/common/wire.rs:61-63; tests/common/sim.rs:106-107; tests/bootstrap.rs:24-27)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'const LINK_BUF' tests/` shows ten definitions; `git log -S` places "matching the sibling bookmark" at 077b64db and "the mirror protocol alternates" at b7fb409f; `src/protocol.rs:15-19` holds only `V2`)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found (the shared constant is younger than three of the suites; causality's 8K rationale was written for the pre-streaming alternating mirror and carried through the V2 port unchanged)
- Owner-gated: no

`common::wire::LINK_BUF` is already `pub const LINK_BUF: usize = 8 * 1024`. Three suites re-declare it at the same value; attach declares `64 * 1024` with no reason, which makes transmit's "matching the sibling bookmark suites" false for one sibling. Causality's rationale names an alternating mirror protocol the crate no longer has. The heal-round cap is spelled three times with two semantics: `MAX_HEAL_ROUNDS_PER_PEER` is multiplied by `n()` at L760, `MAX_HEAL_ROUNDS` is a total at L461-463, and `sim::MAX_QUIESCE_ROUNDS_PER_PEER` is private. Transmit also imports `before::Version` (L37) where its siblings import the re-exported `rumors::Version`. Named constants live once; a hand-maintained cross-reference has already rotted.

Evidence:

        19	/// Capacity for each in-memory link stream carrying a bootstrap session.
        20	const LINK_BUF: usize = 64 * 1024;

    tests/bookmark_transmit_window.rs:
        47	/// Capacity for every in-memory link stream, matching the sibling bookmark
        48	/// suites.
        49	const LINK_BUF: usize = 8 * 1024;

    tests/bookmark_causality.rs:
        78	/// Capacity for every in-memory link stream; the mirror protocol alternates
        79	/// within a session, so a modest buffer suffices and exercises backpressure.
        80	const LINK_BUF: usize = 8 * 1024;

Resolution: import `crate::common::wire::LINK_BUF` in all four suites. If attach needs 64K for a bootstrap payload, state why at the one declaration (tests/bootstrap.rs:24-27 carries such a rationale; retire.rs, gossip_when.rs, reuse.rs, and party_conservation.rs also use 64K, so the crate holds two conventions worth unifying by whoever reviews those partitions). Make `sim::MAX_QUIESCE_ROUNDS_PER_PEER` public and use it for both heal loops. Import `rumors::Version` in transmit. Acceptance: `grep -rn 'const LINK_BUF' tests/bookmark_*.rs` is empty; one heal cap is defined in `tests/common` and imported; no bookmark suite mentions an alternating mirror.

### tests-bookmark-3: Suite-local bootstrap and gossip drivers reimplement `common::wire` for bookmarked peers and lose its drained-control assertion
- Where: tests/bookmark_attach.rs:22-43 (related: tests/bookmark_causality.rs:735-806, 1156-1186; tests/bookmark_transmit_window.rs:148-189, 447-465; tests/bookmark_when.rs:189-227; tests/common/wire.rs:144-158, 244-264; tests/common/sim.rs:879-906; src/rumors.rs:27; src/snapshot.rs:77-79)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`grep -c assert_control_drained` is 0 in all four suites; `a pristine seed attaches` occurs 1+4+4+2 times; `gossip_pair_async` and `bootstrap_fork_configured` take `&Rumors<T>` at wire.rs:144-147 and 244; `Rumors<T, B: BookmarkError = NoBookmark>` at src/rumors.rs:27; transmit's `boot_from` at L150-172 contains no `sync_window_floor`)
- Seen by: structure-prose, api-economics, blind-spots (the floor omission); refutation: confirmed; history: no rationale found (wire.rs took `&Rumors<T>` when the suites were born; 52376b52's drain sweep listed the suites it touched and no bookmark suite is among them)
- Owner-gated: no

The shared drivers exist only over `Rumors<T>` (that is, `NoBookmark`), so every bookmarked suite re-derives them: five bootstrap helpers (`bootstrap_unbookmarked`, `boot_from_async`, `boot_from`, `serve_bootstrap`, `bootstrap_fork_peer`), three clean gossip-pair helpers (`gossip`, `plain_gossip`, `clean_gossip`), two heal loops, and eleven copies of the seed-and-attach chain with the same expect string. The shared drivers call `assert_control_drained` after every successful session; none of the copies do, so the control-drain invariant every other suite enforces is unexercised across every bookmarked session. The copies also drift: transmit's `boot_from` skips `.sync_window_floor()` on the booted peer while attach (L42), causality (L1171), when (L208, L226), and common (`WindowChoice::Floor`, wire.rs:217) pin it, so B, C, and D in the transmit suite run at the default budget; and transmit's heal loop (L450, L456) fingerprints by `snapshot().hash()` alone where `sim::quiesce` and causality use `(hash, latest)`, although `Snapshot::hash`'s own doc says two replicas at different causal points can share a hash. A helper every suite reimplements is a missing harness method, and divergent copies are where coverage gaps hide.

Evidence:

        22	/// Bootstrap a fresh, still-unbookmarked peer from `server` over a clean
        23	/// in-memory link. Both sides run as spawned tasks so a finished one drops
        24	/// its end; the wires are reliable, so the bootstrap succeeds.
        25	async fn bootstrap_unbookmarked(server: &Rumors<String, FlakyInMemoryBookmark>) -> Peer<String> {

    tests/common/wire.rs:
       153	    let (a_result, b_result) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
       154	    let a_report = a_result.expect("wire gossip A");
       155	    let b_report = b_result.expect("wire gossip B");
       156	    assert_control_drained(a_link, b_link);

    tests/bookmark_transmit_window.rs:
       158	        let peer = Peer::<Msg>::bootstrap()
       159	            .join(&mut link)
       160	            .await
       161	            .expect("bootstrap ok")
       162	            .expect("the server is established");
       163	        peer.bookmark(bm).await.expect("in-memory persist")

Resolution: generalize `gossip_pair_async`, `wire_gossip_async`, `bootstrap_fork_configured`, and `sim::quiesce` over `B: Bookmark` (the `Rumors<T, B>` parameter exists and `Rumors<T, B>::gossip` is already generic); add `serve_bootstrap_to<T, B, B2: Bookmark>(server: &Rumors<T, B>, bookmark: B2) -> Rumors<T, B2>` for a newcomer that attaches a bookmark, and `seeded_with<T, B: Bookmark>(bookmark: B) -> Rumors<T, B>` for the pristine-seed attach. Delete the suite-local copies. Keep spawn-based drivers only where spawning is load-bearing (causality's faulted sessions; transmit's gated and aborted sessions, pending finding 11), and route even those through the shared link and drain mechanics. Causality's `clean_gossip` becomes a thin wrapper that calls the driver and then `secure(a); secure(b)`. Acceptance: `grep -n 'async fn boot\|fn bootstrap_\|fn plain_gossip\|fn gossip(' tests/bookmark_*.rs` shows no clean-wire driver bodies; every successful bookmarked session passes through `assert_control_drained`; the pristine-seed expect string appears once, in `tests/common`; every booted peer in the four suites is at the floor.

### tests-bookmark-4: Assertions admit outcomes their docs or intent exclude
- Where: tests/bookmark_attach.rs:45-55 (related: tests/bookmark_transmit_window.rs:363-366, 543-546, 619-622; tests/bookmark_when.rs:436-440 vs 548-552, 466-470, 494, 534-538 vs 556-569, 616-618 vs 627/646/654; tests/common/flaky.rs:148-154)
- Class / severity / confidence: test-quality / nit / high
- Provenance: assessed (read each site; `FaultFeed`'s exhausted queue defaults to success at flaky.rs:149 and 153)
- Seen by: blind-spots, structure-prose, api-economics; refutation: confirmed at every site; history: no rationale found (all birth-state)
- Owner-gated: no

Several assertions are weaker than the sentence above them. `pristine_seed_attaches_without_touching_storage` says "touches no storage" but arms only the write queue, so a spurious `load` at attach would succeed silently. `attaching_to_a_fork_eagerly_persists_then_never_re_reads` says the follow-up session "re-records" but asserts only read counts, and repeats verbatim the `[Read, Write]` assertion `birth` already made at L436-440. `incorporating_remote_content_writes_nothing` asserts `writes >= 1` where the model holds exactly one. `read_is_deferred_to_first_use` indexes `history[0]`, which panics with an index error rather than the message on an empty history. In transmit, the two donation-abort tests accept `Ok(None)` (a mutual-bootstrap bail that cannot occur against a gossiping server) alongside `Err`, and the cancellation check accepts a panicked task as "dropped mid-persist" where `JoinError::is_cancelled` names the intended outcome. `World::apply`'s doc says `None` means the operation was skipped, but the local ops (`Send`, `Redact`, `HelperSend`) also return `None`; the assert message at L731 has it right. An inaccurate testdoc is a bug in the test; an anchor whose exact value is known should assert it.

Evidence:

        49	/// The fault schedule would *fail* a write, so a clean `Ok` is
        50	/// itself proof that none was attempted.
        ...
        55	        let faults = Arc::new(Mutex::new(FaultFeed::new(vec![], vec![true])));

    tests/bookmark_when.rs:
       537	/// That attach read is the lifetime's only read: a following session never repeats
       538	/// it, and (no local work having intervened) it re-records but does not re-read.
        ...
       560	        assert_eq!(
       561	            session.iter().filter(|e| **e == Io::Read).count(),
       562	            0,
       563	            "a bootstrapped peer never reads again after the attach read",
       564	        );

    tests/bookmark_transmit_window.rs:
       363	        assert!(
       364	            out_a.is_err() && out_b.is_err(),
       365	            "both session futures were dropped mid-persist",
       366	        );
        ...
       543	        assert!(
       544	            !matches!(boot_out.unwrap(), Ok(Some(_))),
       545	            "the newcomer must not receive a party the donor could not persist away",
       546	        );

Resolution: attach L55: `FaultFeed::new(vec![true], vec![true])` so a read at attach also fails. when L559-564: assert `session == [Io::Write]`; drop L548-552; L494: `writes == 1`; L466-470: `assert_eq!(history.first(), Some(&Io::Read), ...)`; L616-618: "`None` for local and skipped operations, which must drive no I/O". transmit L543-546 and L619-622: `boot_out.unwrap().is_err()`; L363-366: `out_a.unwrap_err().is_cancelled() && out_b.unwrap_err().is_cancelled()`. Acceptance: each doc sentence maps to an assertion in its body, and each assertion names exactly the outcome the test intends.

### tests-bookmark-5: Doc paragraphs left as a fragment line after a first-sentence split
- Where: tests/bookmark_attach.rs:75-77 (related: tests/bookmark_attach.rs:127; tests/bookmark_causality.rs:212, 310, 472, 492, 647, 1047, 1218; tests/bookmark_when.rs:215, 296, 389)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read every cited line)
- Seen by: structure-prose; refutation: confirmed; history: the split is gate-mandated (`tools/doclint` summary length) and was applied by dfd19c44 without re-wrapping the continuation
- Owner-gated: no

Twelve doc paragraphs open with a one- or two-word line and continue on the next: the signature of a first-sentence split applied without re-wrapping. The split itself is right (the first sentence stands alone in a module listing); the wrap is unfinished. Rendered Markdown hides it; the source does not, and rustfmt does not reflow doc comments.

Evidence:

        75	/// The peer must already *know* something
        76	/// — here, one sent message advancing its frontier — or the pristine-seed
        77	/// shortcut would skip the write the failure rides on.

    tests/bookmark_causality.rs:
       470	    /// Secure what is now known to the network, then discard the rest.
       471	    ///
       472	    /// `who`'s
       473	    /// in-memory state is about to vanish (a crash or a re-bootstrap), so any

Resolution: re-wrap the second paragraph at each listed site. Acceptance: no `///` or `//!` paragraph begins with a line shorter than a clause that does not end a sentence.

### tests-bookmark-6: The attach contract's read-failure and corrupt-frame halves are untested through `Peer::bookmark`
- Where: tests/bookmark_attach.rs:86-96 (related: src/peer.rs:265-271; src/peer/gossip.rs:150-157, 372-398; tests/bookmark_causality.rs:591-593; tests/bootstrap.rs:198-204; src/bookmark/format/tests.rs)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rn BookmarkIo tests/` returns nothing; attach L55, L90, L180 and bootstrap.rs:200-202 inject write faults only; the `# Errors` contract at src/peer.rs:265-271 read)
- Seen by: blind-spots; refutation: reframed (the causality proptest's `read_faults` do reach the attach at L593 in aggregate, but `.ok()` discards the outcome, so nothing checks the contract's shape); history: no rationale found (`BookmarkIo::Format` arrived in 80898f19 with decoder-level coverage only)
- Owner-gated: no

`Peer::bookmark`'s contract covers "cannot be read or written" and promises a specific shape: nothing reaches storage, the peer comes back untouched, and `Unbookmarked.error` is a `BookmarkIo` with an `Io` and a `Format` arm. Every point test injects a write fault. No test drives a `load` failure or a store holding foreign or corrupt bytes through `Peer::bookmark` and checks the peer's party unchanged, the store byte-identical, and the error arm correct. Environmental failures (unreadable or corrupt storage) must be observable as errors of the documented shape (Principle 1); the format decoder's unit tests prove the decode rejects corruption but not that the attach path routes it to `Unbookmarked` without side effects.

Evidence:

        88	        let failing = FlakyInMemoryBookmark::new(
        89	            store.clone(),
        90	            Arc::new(Mutex::new(FaultFeed::new(vec![], vec![true]))),
        91	            0,
        92	        );
        93	        let Unbookmarked { peer, error } = peer
        94	            .bookmark(failing)
        95	            .await
        96	            .expect_err("the injected write failure must surface");

    src/peer.rs:
       267	    /// If the bookmark cannot be read or written, nothing reaches storage and
       268	    /// the peer is handed back **untouched**, still unbookmarked, inside
       269	    /// [`Unbookmarked`], to drop or retry. Because the attach never reclaims, the
       270	    /// live party is exactly as it was: a failed attach cannot leave reclaimed
       271	    /// identity live in this peer yet stranded on disk.

Resolution: add two point tests beside `failed_persist_returns_peer_for_retry`. Acceptance: both tests exist, each names the `BookmarkIo` arm it expects, and a mutant that makes `bookmark_inner` return `Ok(peer)` on a read error fails the first.
Construction: (a) in tests/bookmark_attach.rs, a non-pristine peer (one message sent) attaches a `FlakyInMemoryBookmark` over `FaultFeed::new(vec![true], vec![])`; assert `Err(Unbookmarked { error: BookmarkIo::Io(_), peer })`, `peer.dangerously_alias_party()` equal to the party before, the store still `None`, and a retry over a healthy feed succeeding. (b) Pre-seed the store with `Some(vec![0xff; 8])` (or a frame with foreign magic) and attach the same non-pristine peer; assert `BookmarkIo::Format(_)`, the store bytes byte-identical afterwards, and the party unchanged.

### tests-bookmark-7: Module doc claims wire faults interrupt hand-offs; every hand-off runs on a clean link
- Where: tests/bookmark_causality.rs:10-11 (related: tests/bookmark_causality.rs:514, 518, 563, 581, 673, 721)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'fault::faulty'` in the file returns L514 and L518 only, both inside `gossip`; the bootstrap link at L581 and the retire link at L673 are bare `memory_with_capacity`)
- Seen by: blind-spots; refutation: confirmed; history: no rationale found (inaccurate since birth)
- Owner-gated: no

The doc says wire faults sever sessions "so messages are lost and hand-offs are interrupted", but only `World::gossip` wraps its link in `fault::faulty`; `bootstrap_into` (whose own doc at L563 says "The bootstrap's wire is clean") and `retire` use unfaulted links, and a mismatched gossip resolves through `bootstrap_into`. No party hand-off is ever wire-interrupted in this suite; hand-offs fail only through bookmark faults. The doc credits the `Retire::Uncertain` arm at L721 with coverage that does not exist.

Evidence:

        10	//! - **wire faults** — sessions severed at arbitrary byte offsets (reusing
        11	//!   [`common::fault`]), so messages are lost and hand-offs are interrupted; and
        ...
       673	            let (ret_side, abs_side) = rumors::link::memory_with_capacity(LINK_BUF);

Resolution: either reword the doc to say only plain gossip sessions are wire-faulted and hand-offs are interrupted solely by bookmark faults, or wrap the retire link in `fault::faulty` with a generated `FaultPlan` (as `tests/common/sim.rs::arb_retire` does) while keeping the reliable variant clean so `assert_no_leak` stays sound. Acceptance: the module doc's fault taxonomy matches the link wrapping at L514-518, L581, and L673.

### tests-bookmark-8: The "fully deterministic" schedule claim is false: `Network` ids come from `OsRng` and decide the tie-break
- Where: tests/bookmark_causality.rs:46-56 (related: tests/bookmark_causality.rs:287, 545, 551-554, 639, 745-747, 1225, 1296-1327; src/peer.rs:206-215; src/network.rs:24-25; src/tree/mirror/streaming/tasks.rs:26; src/tree/mirror/streaming/materialized.rs:841; tests/common/wire.rs:49-59)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`Peer::seed()` is `Self::seed_rng(&mut OsRng)` at src/peer.rs:206-208; `Network` derives `Ord` at src/network.rs:24; `grep -rl seed_rng tests/` lists nine suites, none of them bookmark suites; `git log -S'fully deterministic'` resolves to b7fb409f)
- Seen by: blind-spots, api-economics (the `select!` source, merged here as secondary); refutation: confirmed; history: no rationale found (`Peer::seed_rng` predates the suite, cb69fc95, so the suite could have seeded from birth)
- Owner-gated: no

The module doc promises a fully deterministic schedule whose counterexamples replay byte-for-byte and whose shrinking is sound, but every universe is created by `Peer::<Msg>::seed()` (L287, L639, L1225), which draws its `Network` from `OsRng`, and both the mismatch resolution (L545, keyed by `tuple` at L551-554) and the heal winner (L745-747) compare `(min_ticks, network)`. Fresh peers tie at zero ticks, so the winner is a coin flip per run: the same proptest seed can take different paths, a shrunk case can pass on retry, and `reconstructed_cut_gossip_then_retire_under_bookmark_faults` (n = 3, nodes 1 and 2 both pristine at `Gossip(1, 2)`) exercises one of two mismatch resolutions at random. A secondary, plausible-but-undemonstrated source: the streaming mirror's two unbiased `tokio::select!` sites draw tokio's thread-local RNG, and the runtime builder sets no seed. An inaccurate testdoc is a bug in the test, and proptest's shrinking and committed seeds presume replayability.

Evidence:

        48	//! Unlike `disruption.rs`, this simulation runs on a *current-thread* runtime
        49	//! with a fully deterministic, plan-driven schedule (each session is its own
        50	//! `block_on`). The bug class is about the *ordering* of
        51	//! emit/gossip/crash/retire/persist-fail events and the persistence-fault
        52	//! sequence, not watch-channel thread races; determinism makes counterexamples
        53	//! replay byte-for-byte, makes shrinking sound, and makes capturing each
        ...
       287	                let peer = block_on(Peer::<Msg>::seed().sync_window_floor().bookmark(bookmark))
        ...
       545	        let (winner, loser) = if ta >= tb { (a, b) } else { (b, a) };

    src/peer.rs:
       206	    pub fn seed() -> Self {
       207	        Self::seed_rng(&mut OsRng)
       208	    }

Resolution: thread a deterministic RNG through `World` (a `SmallRng` seeded per world, or a counter mapped through `seed_from_u64`) and replace the three `Peer::<Msg>::seed()` calls with `Peer::seed_rng(&mut rng)`; assert generated networks are pairwise distinct. Then the two reconstructed plans pin one path each and can assert which node re-bootstrapped. Calibrate the doc sentence for the `select!` bound ("deterministic up to tokio's `select!` branch order in the session internals") or add a cheap replay-identity check on a fixed plan using `common::fault::metered`'s `ByteMeter`. Acceptance: two runs of `reconstructed_cut_gossip_then_retire_under_bookmark_faults` resolve the mismatch with the same, asserted winner, and the doc claim at L46-56 is true as written.

### tests-bookmark-9: The recycle oracle cannot see the recycle that destroys content
- Where: tests/bookmark_causality.rs:140-146 (related: tests/bookmark_causality.rs:26-28, 96-98, 411-413, 868-893, 1215-1239; src/bookmark.rs:450; src/reconciliation.rs:96-100; crates/before/src/version.rs:51; tests/bookmark_transmit_window.rs:274-305)
- Class / severity / confidence: verification-gap / high / high
- Provenance: assessed (read; traced `reclaim`'s admission test, the sieve's `<=` rule, and `Version`'s order; no test run)
- Seen by: blind-spots; refutation: confirmed, with two refinements to the construction adopted below; history: no rationale found (the version-only argument is birth-state; the reclaim-recycle of 077b64db was pinned in a new suite and nothing records why this suite did not catch it)
- Owner-gated: no

`EmissionLog::promote` flags a recycle only when a later durable emission's full version compares `Less` or `Equal` to an earlier one's. The recycle the bookmark exists to prevent (a rebooted peer reclaims its region below a frontier some replica durably holds, then ticks) yields a version that compares `Greater` or incomparable whenever the reclaimed peer's frontier carries any other region's progress, so the predicate passes it. Because redactions are deliberately untracked (L411-413) and the post-heal check demands a witness only for leaves still live (L868-893), the consequence, the causal sieve deleting the durable message fleet-wide, is invisible too. The module doc's step at L26-28 is the incorrect inference: the version order forbids `later <= earlier`, but a recycle is an own-region collision under a strictly greater or concurrent full version. The suite's negative control (L1221-1239) exercises only the equal-version shape. Mitigation: `record_dominates_the_transmitted_frontier` pins the crate invariant at the transmit boundary, so the crate is covered; the finding is that the proptest's headline claim ("however adverse the run") is judged by an oracle blind to its primary failure class (Principle 6: every criterion needs a committed demonstration that a known-bad mechanism fails it).

Evidence:

       140	            // A recycle is `later <= earlier` in the causal (partial) order;
       141	            // genuinely concurrent versions compare `None` and are fine.
       142	            assert!(
       143	                !matches!(
       144	                    later.version.partial_cmp(&earlier.version),
       145	                    Some(Ordering::Less | Ordering::Equal)
       146	                ),
        ...
        27	//! incomparable, never `<=`). The id-region need not enter the comparison at all
        28	//! — it would only rule out collisions the version order already forbids.

    src/reconciliation.rs:
        96	//! - A leaf whose version is **contained in** (`<=`) the lacking side's
        97	//!   version records a send that side's history already covers: the lacking
        98	//!   side has necessarily *seen* the message, but no longer holds it. Seen
        99	//!   but absent means it must have been deleted — so the holder drops the
       100	//!   leaf locally too, honoring a deletion it was never told about directly.

    src/bookmark.rs:
       450	        for clock in clocks.extract_if(.., |clock| clock.own_version() <= *version) {

Resolution: (1) Track redactions: `World::redact` knows the version it redacts; record it in a per-network ledger (the shape tests/disruption.rs already uses). After `heal`, assert that every message live at some live peer of the winning network at heal start, and never redacted, is live at every peer. Scope the ledger to the winning network: `heal` collapses other universes by re-bootstrapping their members, discarding their content. Under a correct implementation a frontier dominates an emission only by having merged it or a redacter's frontier, so the check is sound; crash-lost content is excluded because it is not live at heal start. (2) Optionally strengthen `promote`: record the emitter's party alias (`dangerously_alias_party()`) on each `Emission` and flag `later.version / later.party <= earlier.version / earlier.party` whenever the two parties overlap; then drop the version-only argument at L26-28 and L96-98. (3) Commit the demonstration: the mutant below passes today's suite and must fail the strengthened one. Acceptance: with the mutant applied to `Bookmarked::reclaim`, `bookmarking_never_recycles_a_version` fails on a plan of the constructed shape; on HEAD both variants stay green across the full case count.
Construction: mutant: in `src/bookmark.rs::reclaim`, push the alias at `Version::new()` instead of `version.clone()` (L467-470), so every stored region is admitted at once on reboot. Plan: n = 3, one network (A = 2, B = 1, C = 0 after bootstraps). `Send(A)` produces m at v_m (ticks only A's region). `Gossip(A, B)` over a clean wire: m propagates, `secure(A)` promotes it (durable). `Send(C)` three times. `Crash(A)`; the next step targeting A calls `revive`, which picks the lowest-index live peer in the network, C (never saw m), and `bootstrap_into` attaches the record (`record` never reclaims). A session `Gossip(A, C)` runs `bookmark_update`, whose `reclaim` admits A's old region under the mutant. `Send(A)` produces v' whose A-region height is at or below m's and whose C-region carries C's three sends: `v'.partial_cmp(&v_m)` is `Greater` or `None`, never `Less | Equal`, so `promote` is silent. `heal`: B meets A'; once A''s frontier dominates v_m and A' lacks m, the sieve deletes m at B; the fleet converges without m; `assert_live_content_is_durable` passes because m is no longer live anywhere. With the ledger from step (1), m is live at B at heal start and unredacted, and the survival assertion fails.

### tests-bookmark-10: Em-dashes in `//` comments and in one assert message
- Where: tests/bookmark_causality.rs:147-149 (related: tests/bookmark_causality.rs:634, 704-705; tests/bookmark_transmit_window.rs:231, 416-417; tests/bookmark_attach.rs:176, 191; tests/bookmark_when.rs:633)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -n '^\s*//[^/!].*—' tests/bookmark_*.rs` returns the nine comment sites; the string literal at L149; `should_panic(expected = "recycled version identifier")` at L1222 matches a suffix and survives a colon)
- Seen by: structure-prose; refutation: confirmed; history: dfd19c44's em-dash sweep covered rendered doc prose only
- Owner-gated: no

An em-dash appears inside the `promote` assert message, where it reaches a terminal, and in nine `//` (non-doc) comments. Doc comments (`///`, `//!`) are rendered prose and are fine. The owner's register rule: colons or semicolons over em-dashes in log messages and code comments.

Evidence:

       147	                "causality violation in network {:?}: durable emission #{} \
       148	                 (version {:?}) is dominated by or equal to earlier durable \
       149	                 emission #{} (version {:?}) — a recycled version identifier",

Resolution: a colon in the assert message; colons or spaced double-hyphens in the listed comments. Acceptance: the grep above and a grep for em-dashes inside string literals in the four files both return nothing.

### tests-bookmark-11: Spawn-based sessions carry no stall bound, so a stall becomes a 180 s nextest kill that loses the proptest reproducer
- Where: tests/bookmark_causality.rs:506-523 (related: tests/bookmark_attach.rs:17, 22-43; tests/bookmark_transmit_window.rs:42, 240-261; tests/bookmark_when.rs:66; tests/common/wire.rs:34-59; .config/nextest.toml:14-26)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`grep -c tokio::spawn`: attach 2, causality 12, transmit 12, when 0; `grep -n timeout tests/bookmark_*.rs` returns nothing; attach, causality, and transmit import `tokio_block_on as block_on`, when imports `block_on`; `.config/nextest.toml:25-26` sets `slow-timeout = { period = "60s", terminate-after = 3 }`)
- Seen by: api-economics; refutation: confirmed, and raised the `join!`-over-`async move` observation adopted in the resolution; history: no rationale found (nextest.toml's collision note is a global ruling silent on per-suite bounds)
- Owner-gated: no

`bookmark_when.rs` drives sessions with `tokio::join!` under `common::wire::block_on`, the closed-world poller, so a wire stall fails immediately at its source. The other three suites use `tokio_block_on` with `tokio::spawn` and never bound a session, so a stall hangs until nextest terminates the process; for the two causality proptests that is exactly the collision nextest.toml documents: a case killed mid-shrink persists no seed. The causality spawn rationale (a faulted side must drop its link to surface EOF) is stated in terms of a `join!` over futures that borrow links declared outside them; a `join!` over two `async move` blocks that each own their link drops a completed block's link when `join!`'s `MaybeDone` transitions, exactly as a spawned task does, so the spawn may have outlived the constraint that justified it (Principle 3). attach's rationale ("the wires are reliable, so the bootstrap succeeds") describes the case the shared `join!`-based drivers already handle.

Evidence:

       506	        // Each side owns its faulted link inside its own task, so when a wire
       507	        // fault kills one side it returns and *drops* its link, surfacing EOF
       508	        // to the counterparty. A bare `join!` would instead hold both sides'
       509	        // links until both finished, deadlocking the survivor on a read that
       510	        // never completes.
       511	        let (out_a, out_b) = block_on(async {

    .config/nextest.toml:
        14	# One collision this budget knowingly accepts: proptest persists a failing
        15	# case's regression seed only after shrinking completes, so a genuinely
        16	# failing property whose shrink phase outruns the 180-second terminate
        17	# budget is killed mid-shrink and the reproducer is lost at exactly the

Resolution: first, one experiment: rewrite `World::gossip` as `block_on(join!(async move { let mut link = fault::faulty(side_a, fault_a); ra.gossip(&mut link).await }, async move { ... }))` under `common::wire::block_on` and run the two reconstructed tests plus a faulted proptest case. If the faulted side's EOF still reaches the survivor, drop `tokio::spawn` and `tokio_block_on` from causality (L506-523, L580-602, L672-694, L786-801) and transmit (the gated session becomes `join!(ga, gb, async { bm_a.entered().await; a.send(M1).unwrap(); bm_a.release() })`; `Notify` needs no runtime), which restores the stall detector without adding a timeout. If it does not, wrap each spawned pair in `tokio::time::timeout(SESSION_BUDGET, join!(...))` (the runtime is built with `enable_all()`) and panic naming the step. Move attach's `bootstrap_unbookmarked` onto the shared driver either way (finding 3). Acceptance: a planted never-completing gossip future in `bookmarking_never_recycles_a_version` fails the case inside the budget and writes a seed to proptest-regressions/bookmark_causality.txt; tests/bookmark_attach.rs imports `common::wire::block_on`, not `tokio_block_on`.

### tests-bookmark-12: The harness swallows session errors that are unconditionally bugs under fault-free regimes
- Where: tests/bookmark_causality.rs:580-602 (related: tests/bookmark_causality.rs:495, 511-535, 563, 621-643, 696-716, 751-754, 802-803, 1054, 1064-1073; tests/common/fault.rs:56-59; tests/common/sim.rs:282-285)
- Class / severity / confidence: test-quality / high / high
- Provenance: demonstrated (second witness pass: under `run_reliable_plan`, every third `World::gossip` was replaced by `Err(Error::PartyOverlap)` on both sides, 452 sessions failed that way across the property's cases, and `bookmarking_prevents_party_leakage` passed; before the pass, assessed by reading: `FaultPlan::is_clean` exists at fault.rs:57-59 and `arb_fault(false)` is `Just(FaultPlan::NONE)` at sim.rs:282-285, both read)
- Seen by: structure-prose, api-economics; refutation: confirmed, with the bound stated below; history: no rationale found (`let _ = serve_out;` is birth-state b7fb409f, whose own message applied the "stays loud" principle to `retire` and `sim` only; the L586 "wire fault" wording is 624917eb drift while L563 "clean" has stood since birth)
- Owner-gated: no

`bootstrap_into` drops the serving task's result, `Result<Result<Gossiped, Error>, JoinError>`, with `let _ = serve_out;` (a panic inside the server's gossip path vanishes) and folds every boot-side error to `None` with `.ok().flatten()?` and `.bookmark(bookmark).await.ok()`. `World::gossip` inspects results only for `NetworkMismatch` and treats every other `Err` as a disruption (L495). `revive` then seeds a fresh universe on a failed rejoin (L633-642), abandoning `single_network`'s one-network premise mid-plan with no assertion. Under `run_reliable_plan` (clean wires, empty feeds, one network) and every `arb_plan` case with `faults = false`, no session can legitimately fail except by `NetworkMismatch`, so any other `Err` is a protocol, codec, or bookmark-format bug; the harness hides it until `heal`, which asserts success and so catches only failures that reproduce there. The masked class is the state-dependent, intermittent one, precisely what the `retire` classifier at L696-716 was written for. `Error::Bookmark(BookmarkIo::Format(_))`, a crate-owned defect since the store returns only bytes the crate wrote, is classified nowhere. The L585-587 comment also names a wire fault as a cause on a wire the helper's own doc calls clean; on this wire the boot side fails only downstream of the server's persist fault (surfacing as a truncated hand-off) or its own attach persist fault. Principle 6: a harness that maps every server-side failure, including panics, to a benign plan outcome lets a crate bug pass as adversity. Bound: persistent failures do surface at heal, and the transmit-window and attach suites cover clean-wire sessions with asserted outcomes.

Evidence:

       585	                // `Ok(None)` cannot happen (the server is gossiping, not
       586	                // bootstrapping); a wire fault drops us to `None`, as does an
       587	                // injected persistence failure in the eager bookmark attach.
       588	                let peer = Peer::<Msg>::bootstrap()
       589	                    .join(&mut link)
       590	                    .await
       591	                    .ok()
       592	                    .flatten()?;
       593	                peer.sync_window_floor().bookmark(bookmark).await.ok()
        ...
       599	            let (boot_out, serve_out) = tokio::join!(boot, serve);
       600	            let _ = serve_out;
       601	            boot_out.expect("bootstrap task")
        ...
       530	        let mismatched = matches!(out_a, Err(Error::NetworkMismatch { .. }))
       531	            || matches!(out_b, Err(Error::NetworkMismatch { .. }));
        ...
       696	        // Never swallow the absorber's result: a retirement's whole point is the
       697	        // hand-off, and silently dropping a failed absorption is exactly what hid
       698	        // the codec leak this test was written to catch. The retire session runs

Resolution: give `World` knowledge of the fault regime (a `reliable: bool` set by `single_network` and by `arb_plan`'s `faults` flag, or `FaultPlan::is_clean` plus an emptiness accessor on `FaultFeed`) and assert `Ok` for gossip, bootstrap-serve, bootstrap-join, and attach whenever the step cannot legitimately fail. Where faults are possible, extract the classifier from `retire` (L706-716) into one `fn assert_not_codec_bug(&Error<FlakyInMemoryBookmark>)` applied to every session result on both sides, extended to `Error::Bookmark(BookmarkIo::Format(_))`, `Error::PartyOverlap`, and `Error::Io` with `InvalidData`; apply it to `Retire::Recovered { error }` and `Retire::Uncertain { error }` too. Bind `serve_out.expect("bootstrap serve task")` so a server panic fails the test. Rewrite L585-587 to name the actual sources on a clean wire. Acceptance: a planted decode failure in the bootstrap serve path fails `bookmarking_prevents_party_leakage` at the offending step with the classified error, not at heal and not never; a deliberately injected `panic!` in a serve-side path fails `bookmarking_never_recycles_a_version` instead of leaving the node dormant; the two proptests and the reconstructed tests still pass unchanged.
Construction: temporarily make `World::gossip` return `Err(Error::PartyOverlap)` from one side on every third call under `run_reliable_plan`; today the leakage property passes because heal's `clean_gossip` (which asserts) runs only after the plan; with the fix, the first such step fails.

Witness: the second witness pass ran this construction (`witness/results.md`, `## tests-bookmark-12`). Two counters were added to tests/bookmark_causality.rs and the session block in `World::gossip` was replaced, on every third call, by `(Err(Error::PartyOverlap), Err(Error::PartyOverlap))` with no session run, a stand-in for any state-dependent non-`NetworkMismatch` failure; then `cargo nextest run -p rumors --all-features --no-capture -E 'test(bookmarking_prevents_party_leakage)'`. The run printed 452 lines of the form `WITNESS tests-bookmark-12: injected Err(PartyOverlap) on both sides, injection #N` (counted with `grep -c`) and ended:

    test bookmarking_prevents_party_leakage ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 3.27s
            PASS [   3.279s] (1/1) rumors::bookmark_causality bookmarking_prevents_party_leakage

The property passed with 452 injected failures because `gossip` inspects results only for `NetworkMismatch` (tests/bookmark_causality.rs:530-531) and `heal`'s `clean_gossip`, which does assert, runs only after the plan; a harness asserting that non-mismatch errors are impossible under the fault-free regime would have failed on the first injection. Bound: only the leakage property was run; the injection hook also sits under `run_plan`, which was not run. The edit was restored afterwards.

### tests-bookmark-13: `heal` pays a confirming full-mesh round that `sim::quiesce` no longer does, and its doc says it mirrors it
- Where: tests/bookmark_causality.rs:758-776 (related: tests/bookmark_causality.rs:734; tests/common/sim.rs:891-898; tests/bookmark_transmit_window.rs:447-465)
- Class / severity / confidence: performance / low / high
- Provenance: verified (read `sim::quiesce` at HEAD and at `0d48b153^` via `git show`; 0d48b153's message names the criterion change and its diff touched bookmark_causality.rs only for doc re-wraps)
- Seen by: api-economics; refutation: confirmed; history: deliberate but expired (0d48b153 changed sim only)
- Owner-gated: no

`World::heal` fingerprints, runs the full mesh, then compares, so a converged fleet pays one mesh (six sessions at n = 4) and a changed fleet pays two. `sim::quiesce` tests all-equal fingerprints before any round and returns without one. Across 2 x 256 cases this is a fixed per-case cost the claim does not need, and "Mirrors `sim::quiesce`" is inaccurate in exactly this respect. The change is strict deletion of redundant work (fixed sign): with equal fingerprints across live peers every pending emission is already propagated, so the trailing `secure` sweep still promotes it.

Evidence:

       734	    /// reachable. Mirrors `sim::quiesce`, plus the network-convergence step.
        ...
       760	        let rounds = MAX_HEAL_ROUNDS_PER_PEER * self.n();
       761	        for _ in 0..rounds {
       762	            let before = self.fingerprints();
       763	            for a in 0..self.n() {
       764	                for b in (a + 1)..self.n() {
       765	                    self.clean_gossip(a, b);
       766	                }
       767	            }
       768	            if self.fingerprints() == before {

    tests/common/sim.rs:
       892	        // Identical fingerprints are the fixed point itself: peers with
       893	        // equal content and version exchange nothing, so no confirming
       894	        // mesh round is owed.
       895	        let first: ([u8; rumors::MERKLE_HASH_LEN], Version) = fingerprint(&peers[0]);
       896	        if peers[1..].iter().all(|p| fingerprint(p) == first) {
       897	            return;

Resolution: at the top of each round, if all live fingerprints are equal, run the final `secure` sweep and return; otherwise run the mesh. Better, once `sim::quiesce` is bookmark-generic (finding 3), call it and keep only the network-collapse step here. Acceptance: a plan whose fleet is converged before heal performs zero heal sessions (a debug counter on `clean_gossip` shows it); both proptests and the reconstructed tests still pass.

### tests-bookmark-14: Asserts implied by the checks before them: the vacuity floor in `assert_live_content_is_durable` and the model self-check in the schedule proptest
- Where: tests/bookmark_causality.rs:885-892 (related: tests/bookmark_causality.rs:160-163, 868-883; tests/bookmark_when.rs:276-278, 284-371, 734-739, 765-775)
- Class / severity / confidence: vestigial / low / high
- Provenance: assessed (read: the loop at L870-883 asserts `contains_exact` for every leaf, so `live_leaves > 0` entails a non-empty log; every `Model` transition that clears `pending` sets `loaded` in the same statement)
- Seen by: structure-prose (both), blind-spots (the causality site); refutation: confirmed both, and rejected structure-prose's alternative floor (every message can be legitimately redacted or crash-lost); history: no rationale found for the causality assert (implied from birth); the model assert's rationale is stated inline but is internal to the model
- Owner-gated: no

Two asserts cannot fail independently of the checks before them. In causality, if `live_leaves > 0` then at least one `contains_exact` returned true, so `emissions.len() >= 1`; the guarded assert is unreachable and is the only caller of `EmissionLog::len`. In when, `pending || loaded` holds by inspection of every `Model` transition (`pristine_seed` sets `pending`, `bootstrap_fork` sets `loaded`, `local_change` sets `pending`, `plain_gossip` and `absorb_retire` set `loaded` where they clear `pending`), so the assert checks test scaffolding against itself; the observable read-before-write property is already asserted on the log at L765-775. A guard earns its place by naming a concrete failure existing checks miss (Principle 3).

Evidence:

       885	        if self.next_seq > 0 && live_leaves > 0 {
       886	            assert!(
       887	                self.emissions.len() > 0,
       888	                "{live_leaves} live message leaves survived after {} sends, but the durable \
       889	                 emission log is empty",
       890	                self.next_seq,
       891	            );
       892	        }

    tests/bookmark_when.rs:
       734	                // The load never lags the first write: a checkpoint is cleared
       735	                // only by a write, and a write is always preceded by the read.
       736	                assert!(
       737	                    world.model.pending || world.model.loaded,
       738	                    "model reached `!pending && !loaded`, which a write cannot produce",
       739	                );

Resolution: delete L885-892 and `EmissionLog::len`; delete when L734-739 and keep the invariant statement in `Model`'s doc at L276-278. Do not replace the causality floor with "`next_seq > 0` implies some node holds content": every sent message can be legitimately redacted or crash-lost. Acceptance: the per-leaf witness loop and the global log checks remain; neither deleted block has a caller.

### tests-bookmark-15: Bookmark fault schedules hold at most seven decisions per node, so late-life faults are unreachable
- Where: tests/bookmark_causality.rs:983-988 (related: tests/common/flaky.rs:116-122, 148-154; src/bookmark.rs:436-464)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read: `0..8` is exclusive; an exhausted queue defaults to success at flaky.rs:149 and 153)
- Seen by: blind-spots; refutation: confirmed, count corrected from eight to seven; history: no rationale found (the length is not derived from a plan's operation count)
- Owner-gated: no

`arb_fault_bits` yields at most seven booleans per node, while a 40-step plan can drive far more bookmark operations per node (each revive is a read and a write; each session up to two writes). Once the queue is exhausted every operation succeeds, so a fault can never land after a node's seventh read or seventh write; a failed write while the record already holds several stranded incarnations (the state the `overlapping`/`covers` filtering in `reclaim` exists for) is outside the generator's reach. A strategy must reach the interesting region; this one front-loads adversity into the first few operations.

Evidence:

       983	fn arb_fault_bits(faults: bool) -> BoxedStrategy<Vec<bool>> {
       984	    if !faults {
       985	        return Just(Vec::new()).boxed();
       986	    }
       987	    prop::collection::vec(prop_oneof![3 => Just(false), 1 => Just(true)], 0..8).boxed()

Resolution: generate fault positions sparsely (a small `BTreeSet<usize>` of failing operation indices over a range that covers a plan's plausible operation count, for example `0..128`), or lengthen the vector to cover it; keep the success bias and the shrink-toward-empty property `FaultFeed`'s exhausted-queue default already provides. Acceptance: a plan can fail a node's bookmark write after that node has completed more than seven prior writes; shrunk minimal counterexamples still shrink toward empty schedules.
Construction: a plan of `Send(0)`, `Crash(0)` repeated nine times (each revive is one read and one write on node 0), then `Gossip(0, 1, NONE, NONE)`: today no generated schedule can fail node 0's tenth write; with a sparse index set containing 9 it can.

### tests-bookmark-16: Test prose narrates the bugs it once caught instead of stating the invariant
- Where: tests/bookmark_causality.rs:1081-1087 (related: tests/bookmark_causality.rs:696-705, 1156-1157, 1263-1267, 1269-1270, 1290-1292; tests/bookmark_transmit_window.rs:313-320; crates/before/src/lib.rs:417)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for `fixed|motivated|Historical|Reconstructed|the fix|Diagnostic` resolves to the cited lines; `mod codec;` is private at crates/before/src/lib.rs:417)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: contradicts AGENTS.md hard rule 1 on its purpose clause ("provenance lives in git history"); the letter enumerates other words, so the owner may read it as Principle 5; "Reconstructed" is 2c73d032's mechanical rename of "Re-minted", not an endorsement of the register
- Owner-gated: no

The regression test's doc tells the story of the `Party::join` defect it caught ("left stale bits", "With the `join` normalization fixed") rather than the invariant and the shape that triggers it, and cites `before::codec`, a private module of a sibling crate. The same register recurs: "exactly what hid the codec leak this test was written to catch" and "the non-canonical party that motivated this check" (L696-705), "Diagnostic helper" on a fixture (L1157), "Historical shrunk counterexamples" (L1263), "Reconstructed counterexample" (L1269, L1290), and "the in-flight-window fix" (transmit L319-320). Prose speaks in the present tense: state what must hold and what shape exercises it; provenance lives in git.

Evidence:

      1081	/// Both peers reclaiming once grew the retiree's donated party via
      1082	/// [`Party::join`](before::Party), which left stale bits in its `as_bytes`
      1083	/// encoding; the absorber's session then aborted decoding it (`before::codec`
      1084	/// `TrailingBits`) while the retiree reported [`Retire::Retired`], having
      1085	/// already shipped and sliced away its party — so the donated region was held
      1086	/// by no one: a leak. With the `join` normalization fixed, the absorption
      1087	/// lands.
        ...
      1263	// Historical shrunk counterexamples for the property above, preserved as
      1264	// explicit constructions: their committed seeds regenerate gossip fault
        ...
       704	        // fully-received frame was malformed — a protocol/codec bug like the
       705	        // non-canonical party that motivated this check — so surface it loudly.

Resolution: rewrite L1075-1092 as the invariant (retiring into an absorber that has itself reclaimed from a bookmark absorbs cleanly and leaves the absorber holding `Party::seed()`), the trigger (both peers have reclaimed; one side alone does not exercise it), and why it lives at the library boundary (no harness). Reword L696-705 to state the classification rule without the incident. Replace L1263-1267 with the live mechanism ("these plans pin the shapes below explicitly; the committed seeds regenerate through the strategy's cut range, so a range change re-maps them") and open L1269 and L1290 with the invariant. Drop "Diagnostic helper" and "the in-flight-window fix" (name the schedule the test does not cover instead). Acceptance: `grep -n 'fixed\|motivated\|Historical\|Reconstructed\|the fix\|Diagnostic helper' tests/bookmark_*.rs` returns nothing.

### tests-bookmark-17: Hand-rolled `.expect`, manual loops where the iterator idiom exists, and small idiom nits
- Where: tests/bookmark_causality.rs:1174-1177 (related: tests/bookmark_causality.rs:396-403, 551, 672 and 695, 941-945, 1054; tests/bookmark_transmit_window.rs:192-198; tests/bookmark_attach.rs:93-96; src/peer.rs:184; src/peer/gossip.rs:149; src/bookmark.rs:127; tests/common/flaky.rs:103; src/lib.rs:330; crates/before/src/party.rs:458)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`Unbookmarked` derives `Debug` at gossip.rs:149, `Peer<T, B>: Debug` at peer.rs:184, `BookmarkIo` derives `Debug` at bookmark.rs:127, `FlakyError` derives `Debug` at flaky.rs:103, so `.expect` compiles; `Ticks` is re-exported at lib.rs:330; `without(self, &Party) -> Option<Party>` at party.rs:458; `awk 'length > 100'` flags only L1054, 117 characters)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: not forced by a missing `Debug` (all four impls existed at 624917eb)
- Owner-gated: no

`match peer.bookmark(bm).await { Ok(peer) => peer, Err(_) => panic!("bookmark ok") }` is `.expect("bookmark ok")` spelled out, and loses the `Debug` payload `.expect` would print (attach L93-96 already uses `expect_err` on the same type). L396-403 finds a leaf version with a manual `for`/`break` loop where transmit's `leaf_version` has the `find().map()` form; L941-945 folds `Party::without` with a manual `take()`/`break` loop that is `held.iter().try_fold(Party::seed(), |rest, region| rest.without(region))`; L695 re-binds a tuple bound at L672 instead of destructuring there; L551 writes `rumors::Ticks` while every other rumors item is imported; L1054 is a 117-character line inside `prop_oneof!`, which rustfmt does not reach.

Evidence:

      1174	        match peer.bookmark(bm).await {
      1175	            Ok(peer) => peer,
      1176	            Err(_) => panic!("bookmark ok"),
      1177	        }

Resolution: `.expect("bookmark ok")`; `snapshot.iter().find(|(_, v)| **v == id).map(|(v, _)| v.clone())` (or share `leaf_version` via `tests/common`); `try_fold` at L941-945; destructure at L672; `use rumors::Ticks`; wrap L1054 by hand. Acceptance: each listed site reads as the one-line idiom; `awk 'length > 100' tests/bookmark_*.rs` is empty.

### tests-bookmark-18: Paired near-duplicates in the causality harness
- Where: tests/bookmark_causality.rs:1188-1196 (related: tests/bookmark_causality.rs:278-305 vs 321-362, 496-535 vs 781-806, 809-818 vs 826-837, 993-1003 vs 1050-1059, 1026-1039 vs 1196-1213)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read each pair)
- Seen by: structure-prose, api-economics (the fingerprint-type item, folded in); refutation: confirmed; history: no rationale found (all birth-state)
- Owner-gated: no

`run_reliable_plan`'s own doc names it as `run_plan` plus one crash guard, and its body repeats the whole `match step` dispatch. Likewise `World::seed` and `single_network` both hand-build the `Node` literal; `clean_gossip` is `gossip` with `FaultPlan::NONE` and results expected; `arb_reliable_step` repeats `arb_step` with other weights; `assert_healed` re-derives the `(hash, latest)` fingerprint that `fingerprints` computes, spelling the tuple type by hand where `let mut reference = None;` would infer it. A 1350-line harness reads faster when each mechanism appears once, and parallel copies invite fix-one-forget-the-other drift.

Evidence:

      1188	/// Execute a reliable-recovery plan to its post-heal end state.
      1189	///
      1190	/// The fleet shares one network from the start, and the only departure from
      1191	/// [`run_plan`] is the crash guard: a crash that would extinguish the network —
      1192	/// leaving no live member for the victim to later reboot from — is skipped, so
      1193	/// that the precondition "every party which restarted eventually restores
      1194	/// itself" holds by construction. (Retirement never extinguishes the network:
      1195	/// the absorber stays live.)

Resolution: add `Node::live(peer, store, faults, label)`; have `gossip` return both outcomes so `clean_gossip` is a call plus two `expect`s; one `fn run(world, steps, guard_crashes: bool)`; `assert_healed` compares `self.fingerprints()` entries pairwise. If the two step strategies must keep distinct weights, parameterize one strategy by a small weights struct, preserving generation order so the committed seeds still replay. Acceptance: each of the five mechanisms has one body; both proptests' committed seeds replay unchanged.

### tests-bookmark-19: Testdoc says "cut in both directions" but both cuts sit on the node-1-to-node-2 direction
- Where: tests/bookmark_causality.rs:1290-1292 (related: tests/bookmark_causality.rs:1304-1311; tests/common/fault.rs:42-47; proptest-regressions/bookmark_causality.txt:8)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `FaultPlan`'s field docs and compared them with the plan literal and the committed seed's plan, which match)
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (6d48d8dc's wording)
- Owner-gated: no

The plan gives node 1 `write_cut: Some(1324)` and node 2 `read_cut: Some(134)`. Per `FaultPlan`, node 1's writes and node 2's reads are the same traffic direction; the node-2-to-node-1 direction is uncut. The doc misdescribes the fixture.

Evidence:

      1290	/// Reconstructed counterexample: a send, one gossip cut in both directions
      1291	/// mid-frame, then a retirement, under bookmark read/write fail
      1292	/// schedules on every node.
        ...
      1304	                FaultPlan {
      1305	                    write_cut: Some(1324),
      1306	                    read_cut: None,
      1307	                },
      1308	                FaultPlan {
      1309	                    write_cut: None,
      1310	                    read_cut: Some(134),
      1311	                },

    tests/common/fault.rs:
        43	    /// Bytes this endpoint may write before its writers fail.
        44	    pub write_cut: Option<usize>,
        45	    /// Bytes this endpoint may read before its readers fail.
        46	    pub read_cut: Option<usize>,

Resolution: reword to "one gossip whose node-1-to-node-2 direction is cut at both ends (the writer after 1324 bytes, the reader after 134), then a retirement, ...". Acceptance: the doc names the direction and both offsets consistently with `FaultPlan`'s field semantics.

### tests-bookmark-20: transmit_window repeats the A-seed prelude four times and the dominance assertion twice, bypassing `Scene`
- Where: tests/bookmark_transmit_window.rs:324-337 (related: tests/bookmark_transmit_window.rs:206-212, 217-270, 286-303, 373-390, 502-511, 583-592)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no rationale found (ccd88401 acknowledged the copy inline and explained why `Scene` could not be reused, not why staging was not factored)
- Owner-gated: no

The seed-A-with-gated-bookmark-then-send-M0 prelude appears verbatim at L220-228, L327-335, L502-510, and L583-591; the comment at L324-326 acknowledges the copy. The record-dominates-transmitted assertion block (party alias, fold of recorded clocks, `b.snapshot().latest()`, two projections, assert) appears at L286-303 and L373-390 differing only in the message. `Scene` exists to carry the staged world but the cancelled-persist test bypasses it because the gate must be armed between staging and the session. Two assertion copies can drift apart in what they project.

Evidence:

       324	        // The scene through the two bootstrap serves, exactly as
       325	        // `transmit_during_persist` stages it: token cleared by the
       326	        // donations, record persisted at M0's frontier.
       327	        let store_a = DurableStore::default();
       328	        let bm_a = GatedBookmark::new(store_a.clone());

Resolution: split staging from the gated session: `async fn stage() -> Scene` (seed A, M0, boot B and C) and `async fn gated_session(&Scene)`; the cancelled-persist test calls `stage()`, sends M1, arms, and aborts. Extract `fn assert_record_dominates_transmitted(a, b, store_a, context: &str)` for L286-303 and L373-390. The two donation tests share a `seed_a_with_b()` prelude. Acceptance: one prelude body and one dominance-assertion body in the file; both tests that need the gate call the same staging function.

### tests-bookmark-21: The destruction check in `restart_after_transmit_never_destroys_durable_messages` is an unreachable arm, and no assertion witnesses a durable message surviving
- Where: tests/bookmark_transmit_window.rs:412-427 (related: tests/bookmark_transmit_window.rs:228, 233-234, 394-399, 468-483)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified for the history (`git log -S'if let Some(version) = m1_at_b'` resolves to 077b64db, "Both red at this commit"; `git log -S'm1_at_b.is_none()'` resolves to ccd88401, "so a schedule drift un-arming its conditional arm fails loudly"); the reading of the body is assessed
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed, structure-prose's framing kept because it names the missing M0 witness; history: the assert is deliberate and its purpose (drift is loud) still holds and is served by the assert alone; the arm is knowingly vacuous residue from the red commit with no recorded reason to remain, and ccd88401 passed its fix-round review without a ruling on it
- Owner-gated: no

The assert at L423 fails first on any drift, so the arm at L475-483 is dead code by construction, and the comment's "goes live only if a change lets a mid-persist commit ride the wire again" is false while the assert stands: on that change the test fails at L423 and the arm still never runs. The doc's headline (L394-395: "A crash after the gated session cannot make the network destroy a live, unredacted message") therefore has no witness in the body. What the body checks is that M1 was not transmitted (a restatement of `record_dominates_the_transmitted_frontier`) and that the fleet reconverges after A restarts from C. M0, the one message durable before the crash (held by B and C from the two bootstraps at L228 and L233-234), is never asserted present after the heal; three replicas converging on content that lost M0 would pass. An inaccurate testdoc is a bug in the test; a branch that cannot execute is scaffolding with no named catch.

Evidence:

       415	        // Under the transmit-window invariant the gated session snapshots
       416	        // its tree before the persist, so M1 — committed inside the persist's
       417	        // in-flight window — stays out of the session and dies, never
       418	        // durable, with A's crash: the destruction arm below is vacuous on
       419	        // this schedule and goes live only if a change lets a mid-persist
       420	        // commit ride the wire again. Asserting the expected outcome makes
       421	        // such drift loud here rather than silently un-arming the check.
       422	        let m1_at_b = leaf_version(&b, M1);
       423	        assert!(
       424	            m1_at_b.is_none(),
       425	            "the gated session must not transmit an own event committed inside the \
       426	             persist's in-flight window",
       427	        );
        ...
       475	        if let Some(version) = m1_at_b {
       476	            for (label, peer) in [("A'", &a2), ("B", &b), ("C", &c)] {
       477	                assert!(
       478	                    leaf_version(peer, M1).is_some(),

Resolution: pick one shape. (a) Recommended: keep the L423 assert as the drift alarm, delete the arm and its comment, add `assert!(leaf_version(peer, M0).is_some(), ...)` for A', B, and C after the heal so the "destroy" claim has a witness, and restate the doc as what is checked (an own event committed inside the persist window is not transmitted; after A crashes and reboots from C the fleet reconverges holding every pre-crash durable message). (b) Positive-path: build a schedule where M1 becomes durable before the crash (send M1, clean gossip A with B, crash A, restart from C which never saw M1, heal) and assert M1 survives everywhere; modest value since the invariant is pinned at the transmit boundary. Acceptance: no branch of the test is unreachable under its own assertions; the doc's first sentence names exactly the assertions in the body; a pre-crash durable message is asserted present on every replica after the heal.

### tests-bookmark-22: Dialect tells: "at the seam", "faithful", "genuine", "honest disruption"
- Where: tests/bookmark_when.rs:1 (related: tests/bookmark_when.rs:32, 80, 585; tests/bookmark_causality.rs:141, 319, 495, 699, 714)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n -i 'the seam\|faithful\|genuine\|honest' tests/bookmark_*.rs` returns exactly the cited lines)
- Seen by: structure-prose, api-economics; refutation: confirmed ("faithful" is the mildest; it describes byte-exactness); history: "seam" is in the owner's writing-style substitution table; the term is repo-wide (48 occurrences in src/, 8 in tests/), so these sites are a slice of a global sweep
- Owner-gated: no

"the seam" is an unanchored metaphor for the `Bookmark` trait boundary; "faithful" moralizes a byte-exact store; "genuinely"/"genuine" and "honest"/"honestly" moralize code where the plain word carries the meaning ("concurrent", "an injected disruption").

Evidence:

         1	//! *When* the identity bookmark is read and written, instrumented at the seam.

Resolution: substitute the mechanism word at each site ("at the `Bookmark` trait boundary", "byte-exact", "concurrent", "an injected disruption") and drop the intensifiers; route "seam" to the repo-wide sweep. Acceptance: the grep above returns nothing.

### tests-bookmark-23: Two suite names collide with crate vocabulary
- Where: tests/bookmark_when.rs:1 (related: tests/bookmark_transmit_window.rs:1-2, 24-30; tests/gossip_when.rs:1-2; src/peer/gossip.rs:160)
- Class / severity / confidence: test-quality / nit / medium
- Provenance: verified (tests/gossip_when.rs:1-2 opens "The [`rumors::Rumors::gossip_when`] driver"; the `window_*.rs` suites and `sync_window_floor` carry the pipeline-window vocabulary)
- Seen by: api-economics; refutation: confirmed; history: no rationale found (gossip_when.rs and the pipeline "window" vocabulary each predate the colliding bookmark suite by days)
- Owner-gated: yes: file names are owner taste

`bookmark_when.rs` sits beside `gossip_when.rs`, whose name refers to the `Rumors::gossip_when` API; a reader expects bookmarks-under-`gossip_when`, but the file is about when bookmark I/O happens. `bookmark_transmit_window.rs` borrows "window", the crate's term for the pipeline budget, for the persist's in-flight interval at the transmit boundary. Names that mislead cost every future reader a redirect.

Evidence:

         1	//! *When* the identity bookmark is read and written, instrumented at the seam.

Resolution: rename to `bookmark_io_schedule.rs` and `bookmark_persist_coverage.rs` (or `bookmark_transmit_coverage.rs`). Neither suite has a seed file, so no seed path moves. Acceptance: neither file name shares a token with a differently scoped crate concept.

### tests-bookmark-24: Rule (2) in the module doc disagrees with `Model::serve_bootstrap` on the session after a donation
- Where: tests/bookmark_when.rs:13-20 (related: tests/bookmark_when.rs:40-55, 341-350; src/bookmark.rs:387-396; src/peer/gossip.rs:564-571)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read the rule, the model transition, `Bookmarked::slice`'s token clearing with its stated hazard, and `bookmark_donate`)
- Seen by: api-economics; refutation: confirmed; history: the implementation policy is deliberate and its rationale lives at src/bookmark.rs:387-394; the finding converts to stating it in rule (2)
- Owner-gated: no for the doc fix; the production alternative reopens ccd88401's design and is listed under open questions

Rule (2) says a session writes only when the peer has unpersisted local work "since its last persist". The donation's own slice-and-write is a persist, after which nothing local has happened, so rule (2) predicts zero writes for the next plain gossip. `Model::serve_bootstrap` sets `pending = true` after the donation write and predicts one write for that session, and the proptest asserts the model. The model is right about the implementation (`slice` clears the token so `bookmark_update` re-records), but that extra write is a policy chosen for a specific hazard (fork-then-absorb returning the party to its pre-donation value at the same version), not a consequence of "what the operation is" as the model section (L40-55) claims. The doc overstates the model's independence and misstates the observable contract.

Evidence:

        13	//! 2. **Write on local work, never on hearsay.** A session persists the record
        14	//!    *before* it transmits, but only when the peer has unpersisted *local*
        15	//!    identity work to checkpoint: it has never persisted, or has emitted a local
        16	//!    change (a [`send`](rumors::Rumors::send) or
        17	//!    [`redact`](rumors::Rumors::redact)) or moved a party (donated a fork,
        18	//!    absorbed a retiree) since its last persist. Merely *incorporating*
        19	//!    content learned over gossip advances only other parties' identities,
        20	//!    never the peer's own, so it triggers no write at all.
        ...
       344	    fn serve_bootstrap(&mut self) -> Delta {
       345	        let reads = self.read_on_first_use();
       346	        let writes = 1 + usize::from(self.pending);
       347	        self.loaded = true;
       348	        self.pending = true;
       349	        Delta { reads, writes }
       350	    }

    src/bookmark.rs:
       387	        // Donating shrinks our live identity, so the suppression token is now
       388	        // stale: clear it. Leaving it would let a later update wrongly suppress
       389	        // if the party happened to return to its pre-donation value at the same
       390	        // version (e.g. forking for a bootstrap, then absorbing that peer's

Resolution: state the post-donation re-record explicitly in rule (2) as policy ("a donation persists the sliced record but leaves no suppression token, so the session after a donation re-records the live identity once"), and in `Model::serve_bootstrap`'s doc name the hazard the policy guards. Acceptance: rule (2), the model-section prose, and `Model::serve_bootstrap` agree on the post-donation session; a reader can predict the `Delta` from the doc alone.

### tests-bookmark-25: `Instrument::pristine_seed` duplicates `birth(Origin::Seed)`; `Birth`, `World`, and `Instrument` overlap; the read/write count is spelled out seven times
- Where: tests/bookmark_when.rs:138-160 (related: tests/bookmark_when.rs:168-175, 393-398, 410-426, 608-613, 707-716, 749-757; the filter expression at 172, 472, 528, 561, 566, 754, 768)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read; the inline at L750-757 is forced because `retire_subject(world.probe.subject, ...)` at L749 partially moves `world.probe`)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found (birth-state bb6ce27e)
- Owner-gated: no

`Instrument::pristine_seed` is `birth(Origin::Seed)` with the same expect and assert messages. `Birth` is `Instrument` plus `model` and `helpers`; `World` is the same plus `next_msg`; the proptest rebuilds an `Instrument` from `Birth`'s parts. The expression `.iter().filter(|e| **e == Io::Read).count()` appears seven times, and L750-757 re-implements `counts_since` inline because the method receiver has been moved out: the tell that the count belongs on the log, not the instrument.

Evidence:

       146	            let subject = Peer::<u64>::seed()
       147	                .sync_window_floor()
       148	                .bookmark(probe)
       149	                .await
       150	                .expect("a pristine seed attaches without touching storage");
        ...
       411	            let subject = Peer::<u64>::seed()
       412	                .sync_window_floor()
       413	                .bookmark(probe)
       414	                .await
       415	                .expect("a pristine seed attaches without touching storage");
        ...
       750	                let (reads, writes) = {
       751	                    let log = log.lock().unwrap();
       752	                    let slice = &log[cursor..];
       753	                    (
       754	                        slice.iter().filter(|e| **e == Io::Read).count(),
       755	                        slice.iter().filter(|e| **e == Io::Write).count(),
       756	                    )
       757	                };

Resolution: make `birth` return `World` (or fold `Birth` into `World`) and define `Instrument::pristine_seed` as `birth(Origin::Seed)`; add `fn delta(log: &[Io]) -> Delta` and use it in `counts_since`, the anchor tests, the retire tail, and the global read-once check. Acceptance: one construction path per origin; `filter(|e| **e == Io::Read)` appears once in the file.

### tests-bookmark-26: The when-model's alphabet has no no-op redact and no helper redaction
- Where: tests/bookmark_when.rs:629-655 (related: tests/bookmark_when.rs:578-605; src/rumors.rs:252-259)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read the op alphabet and `apply`; `redact_all`'s doc at src/rumors.rs:258 states "versions not currently held are skipped")
- Seen by: blind-spots; refutation: confirmed for the coverage gap (the reader's mechanism claim about `act.rs` was not adopted); history: no rationale found (bb6ce27e models only "a redact that removed a held key")
- Owner-gated: no

`Op::Redact` only ever redacts a version the subject currently holds and is skipped on an empty set, so the contract's unheld-version no-op is never checked against the bookmark schedule: a change that ticked on an ineffectual redaction would cause a write the model does not predict, and nothing here would see it. No helper ever redacts, so the subject never incorporates a ceiling-only remote advance (a frontier move with no content change), the one remote-content shape that differs from `HelperSend`. These are the two edges of the "write on local work, never on hearsay" line, and neither is in the alphabet.

Evidence:

       629	            Op::Redact(i) => {
       630	                // Redacting a message the application currently holds always
       631	                // records a deletion in the subject's own region, ticking it;
       632	                // redacting nothing (an empty set) is a true no-op. Liveness
       633	                // is read from the snapshot — the application's own view —
       634	                // never from the version arithmetic the suppression uses.

Resolution: add `Op::RedactAgain` (re-redact the most recently redacted version; model: no local change, so the next session drives no I/O) and `Op::HelperRedact(usize)` (a helper redacts one of its own held messages; model: pure hearsay). Both are a few lines in `apply` and the strategy. Acceptance: the proptest alphabet includes both ops and the model's predictions still match at every step.
Construction: with `Op::RedactAgain` in the alphabet, a mutant `Rumors::redact` that ticks the own region on an unheld version produces `Delta { writes: 1 }` at the next `Gossip` where the model predicts `writes: 0`, and the proptest fails.

## Positives

- tests/bookmark_causality.rs:696-716: `retire` refuses to swallow the absorber's result and splits decode failures (`InvalidData`, `HandOffMalformed`) from injected disruptions with the reasoning inline. This is the standard the rest of the harness should be held to (finding 12 asks only that it be applied uniformly).
- tests/bookmark_causality.rs:121-158 and 1221-1239: the recycle check runs as the durable set grows so a failure lands on the most-shrunk witness, and a `should_panic` negative control proves the verifier rejects a recycled coordinate.
- tests/bookmark_causality.rs:895-951: `assert_no_leak` is stated as the exact dual of live-party disjointness (claimed zero times versus claimed twice), with a clear argument for why checkpointed-but-not-live regions count as held.
- tests/bookmark_when.rs:40-55 and 261-371: the `Model` predicts I/O from operation semantics alone and never from the crate's suppression arithmetic, so the proptest is a differential oracle rather than the implementation compared with itself; per-step deltas plus a global read-once/read-before-write check.
- tests/bookmark_transmit_window.rs:57-144: `GatedBookmark` turns a durable-write/commit race into a deterministic, replayable interleaving point with `Notify`. The history is instruments-before-cures done right: 077b64db committed `record_dominates_the_transmitted_frontier` red, and ccd88401 committed `cancelled_persist_never_suppresses_the_next_update` "red before this fix" (both commit messages verified); the latter exercises the cancel-safety clause of `Bookmarked::write` through the public API only.
- proptest-regressions/bookmark_causality.txt holds both seeds and they match the explicit reconstructions at tests/bookmark_causality.rs:1276-1286 and 1297-1325 exactly (verified by comparing the plans); the comment at L1263-1267 gives a real reason (strategy range changes re-map cut offsets) for keeping both.
- All four suites drive the public API plus the gated `test-internals` aliases (`dangerously_alias_party`, `sync_window_floor`); no internal protocol entry is exercised, so coverage cannot drift when the public wiring changes.
- tests/bookmark_attach.rs:132-138 and 139-210: `failed_attach_does_not_reclaim_into_an_unbookmarked_peer` constructs the exact recycle shape (a fresh fork attached to a store still holding a dominated previous incarnation over a failing write) rather than arguing it, and asserts both the live party and the on-disk record.
- The runtime choice is stated and accurate where it is stated: tests/bookmark_causality.rs:48-50 correctly contrasts its current-thread runtime with tests/disruption.rs's multi-thread runtime (disruption.rs:44-49 verified), and the three suites that spawn tasks each say why; the when suite uses the quiescence-checked `block_on` so a protocol stall fails at its source.
- tests/common/flaky.rs:210-215 (the harness these suites rest on): `FlakyInMemoryBookmark::store` fails before touching the durable bytes, modelling exactly the atomic-commit obligation the `Bookmark` trait places on implementors, so recovery paths are tested against the contract rather than a lenient stand-in.

## Open questions for Finch

- Generalizing `common::wire`'s drivers (finding 3): generalize `gossip_pair_async`, `bootstrap_fork_configured`, and `sim::quiesce` over `B: Bookmark` (touching a module every suite shares), or land a bookmark-specific driver module beside them? Recommendation: generalize; the `Rumors<T, B>` parameter already exists, and one driver family keeps the drain assertion in one place.
- `LINK_BUF` conventions (finding 2): attach, retire, bootstrap, gossip_when, reuse, and party_conservation use 64K; `common::wire` and the other bookmark suites use 8K. Is 64K a deliberate convention for bootstrap-carrying sessions? Recommendation: import common's 8K in the bookmark suites now; if 64K is load-bearing for bootstrap payloads, name it once in `tests/common` with bootstrap.rs's rationale and have those six suites import it (cross-partition).
- The destruction-class oracle (finding 9): extend the causality simulation with a redaction ledger and a survival check, or accept that the random simulation pins only the degenerate recycle and leave the destruction class to constructed schedules in the transmit suite? Recommendation: the ledger, because the proptest's headline claim is the recycle property and it should be judged by an oracle that can see the primary class; the mutant demonstration then earns its place as a committed check.
- A yielding store in the causality `World` (raised by blind-spots, no finding): the random simulation cannot reach the mid-persist-send cause because `FlakyInMemoryBookmark::store` completes without yielding and the `World` never sends concurrently. Recommendation: not now; the transmit suite owns that class by construction, and the causality module doc should say the mid-persist window is out of its scope.
- Shape of finding 21: negative-space (keep the drift alarm, delete the arm, add the M0 survival witness) or positive-path (make M1 durable and assert it survives)? Recommendation: negative-space; it is accurate about the schedule and costs three lines.
- Suite renames (finding 23): `bookmark_io_schedule.rs` and `bookmark_persist_coverage.rs`? Owner taste; recommendation: rename, since both collisions are with live crate vocabulary.
- The post-donation re-record (finding 24): `Bookmarked::slice` could stage `(sliced party, recorded version)` as the token so the redundant write disappears while fork-then-absorb still differs from the token; the version would have to be the one already in the record, never the live one, or the transmit-window invariant is breached. This reopens ccd88401's "token commits only with the write" design. Recommendation: leave the policy and fix the doc; raise the alternative with the production reviewer only if the extra write per donation matters.
- One binary or four (raised by api-economics, no finding): each `tests/*.rs` links `common` separately; merging the four bookmark suites into `tests/bookmark.rs` with submodules would save compile and link time and relocate the seed to `proptest-regressions/bookmark/causality.txt` under the `tests/main.rs` anchor. Recommendation: measure link time first; do not act on this review alone.

## Dropped

- [0] `bootstrap_into` discards the serving side's outcome: merged into tests-bookmark-12 (same site; that finding covers `gossip` and `revive` too).
- [1], [19], [29] the dead destruction arm (three lenses): merged into tests-bookmark-21, keeping structure-prose's framing because it names the missing M0 witness.
- [2], [30] suite-local drivers (two lenses): merged into tests-bookmark-3, with blind-spots' floor omission [27] and the refutation's hash-only heal fingerprint folded in.
- [3] trailing assert cannot fire: merged into tests-bookmark-14 with [16] (same principle, two files); the refutation rejected [3]'s proposed replacement floor and so do I.
- [4], [37] `LINK_BUF` and heal cap (two lenses): merged into tests-bookmark-2, with transmit's `before::Version` import folded in.
- [5], [27] (staging half), [38] (transmit items): merged into tests-bookmark-20.
- [6], [38] (when items): merged into tests-bookmark-25.
- [10], [25] (provenance half), [35]: merged into tests-bookmark-16.
- [15], [24] (when and transmit items), [25] (the re-records half), [26]: merged into tests-bookmark-4 as one pattern (assertions weaker than their docs or intent).
- [34] `select!` nondeterminism: merged into tests-bookmark-8 as the secondary, undemonstrated source; the `OsRng` source is the definite falsifier.
- [39] `Snapshot::hash` newtype: reframed by the refutation to a test-side alias or inference; the test-side item is folded into tests-bookmark-18 and the API question is below the bar for this partition (no test fights the type once `reference` infers).
- [40] `let _ = self.label;` in tests/common/flaky.rs:199: verified (the field is read by the `Debug` impl at L172-178 and `tests/common/mod.rs:33` allows dead code), but the file is outside this partition; route to the tests/common finalizer.
- Refutation NEW 5 (a scattered seed file at 077b64db^): nothing to do at HEAD; `tests/seed_liveness.rs` guards the shape.
- Blind-spots open question on a yielding store, api-economics open questions on `sync_window_floor` everywhere and on merging binaries: not findings; the floor pin is a crate-wide convention (db2718d4) already ruled, and the other two are recorded above as owner questions.
