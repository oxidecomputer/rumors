//! Substantiates the docstring claim on [`rumors::Local::for_party`] that
//! reusing a party identifier non-causally — either two concurrent processes
//! sharing a party tag, or a single process restarted at a `start` strictly
//! less than the prior instantiation's last [`event()`](rumors::Local::event) —
//! can lead to arbitrary, contagious corruption of the rumor set.
//!
//! # The mechanism
//!
//! Two ingredients combine to produce the corruption:
//!
//! 1. **Leaf paths are content-addressed by `Hash(party, scalar, value)`** (see
//!    [`src/tree/typed/path.rs::Path::for_leaf`]). When two disjoint processes
//!    write different values under the same `(party, scalar)`, the resulting
//!    leaves occupy *different* paths in the tree, but each carries a version
//!    vector whose `party`-component equals the same `scalar`.
//!
//! 2. **The mirror protocol uses version-vector dominance as its
//!    "they-have-it-or-deleted-it" oracle.** Concretely:
//!      - [`src/tree/traverse/mirror/local.rs::Exchange::accept`] short-circuits
//!        the whole exchange with `Step::Done` whenever `our_version ==
//!        their_version`.
//!      - At every subtree boundary [`Exchange::answer_requested`] /
//!        [`Exchange::partition_uncertain`] feed the surviving subtree through
//!        [`tree::traverse::unknown::unknown`], which drops any subtree whose
//!        `version <= their_version` on the grounds that "anything causally
//!        prior to it that they lack, they have already deleted -- so we
//!        should too" (cf. the comment at local.rs:561). This filter applies
//!        symmetrically to *both* what we send (we stop offering subtrees the
//!        peer "deleted") and what we keep locally (the surviving subtree
//!        replaces our own copy, so leaves the peer "deleted" get pruned out
//!        of our tree as well).
//!
//! Under (1) and (2), each life's leaf lives at a tree path the other life
//! has never heard of, but carries a version-vector entry that the other
//! life's tree pointwise dominates. The filter therefore mistakes
//! "they never saw it" for "they saw and redacted it", so:
//!
//! - while the two lives' version vectors remain equal, the
//!   [`accept`-time](src/tree/traverse/mirror/local.rs) early exit fires and
//!   no exchange happens — both colliding leaves are stranded on opposite
//!   sides of the divergence;
//! - the moment one side advances its scalar past the collision point, the
//!   next exchange filters the colliding leaves through the
//!   forget-inference path on *both* sides at once, destroying the messages
//!   network-wide.
//!
//! # What this file proves
//!
//! Two complementary tests walk the corruption from the narrowest to the
//! broadest case the protocol permits.
//!
//! [`reused_party_corrupts_rumor_set`] is the minimal violation: a restart
//! that violates `start >= event()` by exactly 1, with one colliding insert
//! on each side. It establishes the four observable corruption modes:
//!
//! - **Silent divergence.** Immediately after the restart, neither peer
//!   learns the other's colliding message; the gossip API reports success.
//! - **Data destruction.** The very next exchange after the restarted party
//!   advances its scalar wipes both colliding messages out of *both* peers'
//!   trees: the protocol reads each peer's missing copy as the peer's
//!   redaction and propagates the redaction back.
//! - **Contagion.** Fresh peers that gossip with the corrupted ones inherit
//!   the corrupted view; no further gossip can resurrect the destroyed
//!   messages.
//! - **Locally undetectable.** Every peer's local view is internally
//!   coherent; nothing surfaces as an error or warning.
//!
//! [`scalar_inflation_destroys_unrelated_alice_history`] generalises the
//! claim to its full blast radius. Because [`Unknown::unknown`] applies the
//! same `subtree.version <= their_version` filter at *every* subtree on
//! *both* peers, a reused-party process whose second life inflates its
//! scalar past the legitimate prior frontier causes *every alice-tagged
//! subtree dominated by the inflated scalar to be pruned* — destroying
//! unrelated history that never collided in path, never shared content with
//! any reuse-life insert, and was originated entirely by the prior life.
//! The test makes the destruction quantitative (`M` prior messages obliterated
//! from a single restart) and pins down the only protection mechanism the
//! protocol offers: a foreign-party leaf in the same subtree contributes a
//! version-vector axis the reuser cannot dominate, so that subtree's
//! contents survive. Whether any given prior message is protected is
//! therefore a function of Blake3's distribution of leaf paths into
//! 256-ary radix subtrees — a "particular shape of hash prefix" that
//! decides whether each leaf shares a subtree with a foreign-axis
//! companion. With random message content the typical fate of every
//! alice-tagged leaf in the dominated range is destruction.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use rumors::sync::{Local, ignore};

/// Project a peer's currently-live messages into a `value -> count` multiset
/// by mirroring it into a throwaway empty `Local`. Forgotten entries do not
/// fire `on_message` and are naturally excluded. Mirrors the
/// `tests/common/oracle.rs::readout_multiset` helper but inlined so this
/// regression stays self-contained.
fn live_multiset<Id>(peer: &Local<String, Id>) -> BTreeMap<String, usize> {
    let observed: Arc<Mutex<BTreeMap<String, usize>>> = Arc::new(Mutex::new(BTreeMap::new()));
    let observed_in = Arc::clone(&observed);
    // Non-ASCII party tag cannot collide with the human-readable ASCII tags
    // the test uses.
    let mut lens = Local::<String, _>::for_party(b"\x00LENS\x00party_identity_reuse", 0).unwrap();
    lens.process(peer.fork(), move |_, _, m: &Arc<String>| {
        *observed_in
            .lock()
            .unwrap()
            .entry(String::clone(m))
            .or_insert(0) += 1;
    });
    Arc::try_unwrap(observed)
        .ok()
        .expect("lens callback dropped after `process` returns")
        .into_inner()
        .expect("mutex not poisoned")
}

/// Bidirectional `process`-based gossip step. The sync API's `process` is a
/// faithful in-process model of [`Local::gossip`] — both run the same mirror
/// protocol; only the framing differs — so the corruption proven here on
/// `process` applies verbatim over the wire.
fn gossip<A: Send, B: Send>(a: &mut Local<String, A>, b: &mut Local<String, B>) {
    let a_snap = a.fork();
    let b_snap = b.fork();
    a.process(b_snap, ignore);
    b.process(a_snap, ignore);
}

/// Restart Alice with `start = 0` after she has already emitted one message
/// (violating the `start >= event()` contract by exactly 1) and show that the
/// resulting rumor set is silently corrupted, that the corruption destroys
/// data on a subsequent exchange, and that the destruction is contagious
/// to every peer.
#[test]
fn reused_party_corrupts_rumor_set() {
    // ----------------------------------------------------------------
    // Phase 1 — Alice's first life publishes one message and tells Bob.
    // ----------------------------------------------------------------
    let mut alice = Local::<String, _>::for_party("alice", 0).unwrap();
    alice.message(["from-life-1".to_string()], ignore);
    assert_eq!(
        alice.event(),
        1,
        "one insert should advance the local scalar to 1"
    );

    let mut bob = Local::<String, _>::for_party("bob", 0).unwrap();
    gossip(&mut alice, &mut bob);
    assert_eq!(
        live_multiset(&bob).get("from-life-1"),
        Some(&1),
        "bob should learn the message Alice originated in life 1"
    );

    // ----------------------------------------------------------------
    // Crash. The application forgot to persist `event()`, so on restart
    // it (incorrectly) hands `start = 0` back to `for_party`.
    // ----------------------------------------------------------------
    drop(alice);
    let mut alice = Local::<String, _>::for_party("alice", 0).unwrap();
    assert_eq!(
        alice.event(),
        0,
        "the restarted Alice has no memory of life 1's scalar"
    );

    // Life 2 publishes a *different* message. Because `event` was reset,
    // the new leaf gets stamped at the same `(party=alice, scalar=1)` as
    // Bob's existing "from-life-1" leaf — but at a *different* tree path,
    // because the path mixes in the value (see `Path::for_leaf`).
    alice.message(["from-life-2".to_string()], ignore);
    assert_eq!(
        alice.event(),
        1,
        "the restarted Alice's first insert collides on `(alice, 1)`"
    );

    // ----------------------------------------------------------------
    // Phase 2a — first gossip after the restart. Version vectors agree
    // pointwise (both peers have `(alice, 1)`), so the mirror protocol's
    // `accept`-time early-exit fires and *nothing* is exchanged. Each
    // peer keeps its own colliding leaf; neither learns the other's.
    // The gossip API reports success.
    // ----------------------------------------------------------------
    gossip(&mut alice, &mut bob);

    let alice_live = live_multiset(&alice);
    let bob_live = live_multiset(&bob);
    assert_eq!(alice_live.get("from-life-2"), Some(&1));
    assert_eq!(bob_live.get("from-life-1"), Some(&1));
    assert!(
        !alice_live.contains_key("from-life-1"),
        "early-exit on equal version vectors hid life-1 from the restarted Alice"
    );
    assert!(
        !bob_live.contains_key("from-life-2"),
        "early-exit on equal version vectors hid life-2 from Bob"
    );

    // ----------------------------------------------------------------
    // Phase 2b — the very next exchange *destroys data*. Alice posts
    // a fresh message, advancing her scalar to `(alice, 2)` and breaking
    // the version-vector equality. The mirror protocol now runs in full,
    // and at each subtree it asks "is this subtree present on my peer?"
    // — but reads "my peer's version dominates the subtree, yet the
    // subtree is absent" as "my peer has forgotten this subtree".
    //
    // The two colliding `(alice, 1)` leaves trigger that path
    // symmetrically on both sides: Alice sees Bob lacking "from-life-2"
    // at `(alice, 1)` and prunes it; Bob sees Alice lacking
    // "from-life-1" at `(alice, 1)` and prunes it. The only message
    // surviving the exchange is the freshly-inserted one, which
    // straddles a `scalar` above the collision.
    // ----------------------------------------------------------------
    alice.message(["from-life-2-bis".to_string()], ignore);
    assert_eq!(alice.event(), 2);
    gossip(&mut alice, &mut bob);

    let alice_live = live_multiset(&alice);
    let bob_live = live_multiset(&bob);
    assert_eq!(
        alice_live,
        BTreeMap::from([("from-life-2-bis".to_string(), 1)]),
        "Alice's tree was wiped of its own life-2 leaf by forget-inference"
    );
    assert_eq!(
        bob_live,
        BTreeMap::from([("from-life-2-bis".to_string(), 1)]),
        "Bob's tree was wiped of its life-1 copy by forget-inference"
    );

    // ----------------------------------------------------------------
    // Phase 3 — contagion. A previously-uninvolved peer Carol gossips
    // with the survivors. She inherits exactly the corrupted view: the
    // destroyed messages are now gone *everywhere*, and no third party
    // can ever encounter them. The corruption has propagated.
    // ----------------------------------------------------------------
    let mut carol = Local::<String, _>::for_party("carol", 0).unwrap();
    gossip(&mut alice, &mut carol);
    gossip(&mut bob, &mut carol);

    let carol_live = live_multiset(&carol);
    assert_eq!(
        carol_live,
        BTreeMap::from([("from-life-2-bis".to_string(), 1)]),
        "Carol inherits the corrupted view; the destroyed messages are gone network-wide"
    );

    // And there is no way back. Even if a future peer somehow had a
    // copy of one of the destroyed messages at version `(alice, 1)`,
    // every other peer's `alice`-scalar now sits at 2, so the next
    // exchange would forget-infer that copy out too. The data loss is
    // permanent for the lifetime of every participant in the network.
}

/// The corruption is not confined to the colliding `(party, scalar)` pair.
/// Once a reused party's scalar is *inflated* past the legitimate prior
/// frontier, the [`Unknown::unknown`] filter applies at every subtree of the
/// network and prunes *every* subtree whose joined version is dominated by
/// the inflated counterparty — destroying arbitrary unrelated history that
/// happens to share the reused party's tag.
///
/// Concretely: Alice's first life originates `M` legitimately-distinct
/// messages and gossips them to Bob. After a non-causal restart, Alice's
/// second life originates `N > M` new messages, inflating her `alice`-scalar
/// to `N`. The next exchange with Bob then reads *every* alice-tagged leaf
/// in Bob's tree as a redaction by Alice — because each such leaf sits in a
/// subtree whose joined version is purely on the `alice` axis and
/// pointwise-dominated by `(alice, N)` — and prunes the lot.
///
/// The destruction is shaped by the Blake3 paths: a subtree gets a free pass
/// if it contains *any* leaf whose party-axis Alice's version vector cannot
/// dominate. We pin that protection down by having Carol publish an
/// independent message at `(carol, 1)`; Bob's carol-tagged subtree survives
/// the exchange untouched, while every one of his alice-tagged subtrees is
/// wiped.
#[test]
fn scalar_inflation_destroys_unrelated_alice_history() {
    const M: usize = 4; // life-1 messages — none ever collide on `(party, scalar)`
    const N: usize = 8; // life-2 messages — inflate `alice`-scalar to `N` > `M`

    // ----------------------------------------------------------------
    // Phase 1 — Alice's first life originates M messages and tells Bob.
    // Then Carol independently posts one message, which Bob also picks
    // up. Bob's tree now holds M alice-tagged leaves plus one
    // carol-tagged leaf, all uncontroversial.
    // ----------------------------------------------------------------
    let mut alice = Local::<String, _>::for_party("alice", 0).unwrap();
    let life_1: Vec<String> = (1..=M).map(|i| format!("life-1-msg-{i}")).collect();
    alice.message(life_1.clone(), ignore);
    assert_eq!(alice.event(), M as u64);

    let mut bob = Local::<String, _>::for_party("bob", 0).unwrap();
    gossip(&mut alice, &mut bob);

    let mut carol = Local::<String, _>::for_party("carol", 0).unwrap();
    carol.message(["carol-news".to_string()], ignore);
    gossip(&mut carol, &mut bob);

    let bob_live = live_multiset(&bob);
    for msg in &life_1 {
        assert_eq!(bob_live.get(msg), Some(&1), "bob should hold {msg}");
    }
    assert_eq!(bob_live.get("carol-news"), Some(&1));
    assert_eq!(bob_live.len(), M + 1, "bob's preconditions");

    // ----------------------------------------------------------------
    // Crash + restart with `start = 0`, violating `start >= event()`.
    // ----------------------------------------------------------------
    drop(alice);
    let mut alice = Local::<String, _>::for_party("alice", 0).unwrap();

    // ----------------------------------------------------------------
    // Alice life 2 publishes N > M brand-new messages, inflating her
    // `alice`-scalar from 0 to N. None of these messages duplicate a
    // life-1 content string; the only "collision" is that their version
    // tags cover the range `(alice, 1) ..= (alice, N)`, which overlaps
    // life-1's `(alice, 1) ..= (alice, M)`.
    // ----------------------------------------------------------------
    let life_2: Vec<String> = (1..=N).map(|i| format!("life-2-msg-{i}")).collect();
    alice.message(life_2.clone(), ignore);
    assert_eq!(alice.event(), N as u64);

    // ----------------------------------------------------------------
    // The Big Exchange. Both filters fire:
    //
    // - On Bob's side, every alice-tagged subtree has version
    //   `(alice, k)` for some `k <= M < N`, so the filter against
    //   Alice's `(alice, N)` prunes them all. Bob loses *all M of his
    //   alice-life-1 messages*, none of which collided in path with any
    //   life-2 message; they are unrelated history collateralized by
    //   the reused party tag.
    //
    // - On Alice's side, her own life-2 leaves at scalars `1..=M` sit
    //   in subtrees whose version `(alice, k)` for `k <= M` is also
    //   dominated by Bob's `(alice, M, carol, 1)`, so they are pruned
    //   too. Alice retains only her life-2 leaves at scalars `M+1..=N`.
    //
    // - Carol's leaf at version `(carol, 1)` survives on Bob's side
    //   because its containing subtree carries a `carol`-component
    //   that Alice's version vector cannot dominate, and is shipped
    //   across to Alice.
    // ----------------------------------------------------------------
    gossip(&mut alice, &mut bob);

    let alice_live = live_multiset(&alice);
    let bob_live = live_multiset(&bob);

    // Every alice-life-1 message is gone from Bob — *the colliding
    // scalar wiped out every legitimate prior message tagged with the
    // reused party, despite none of them sharing a leaf path with any
    // life-2 insert*.
    for msg in &life_1 {
        assert!(
            !bob_live.contains_key(msg),
            "scalar inflation destroyed unrelated alice-life-1 message {msg} on Bob"
        );
        assert!(
            !alice_live.contains_key(msg),
            "scalar inflation destroyed unrelated alice-life-1 message {msg} on Alice"
        );
    }

    // The low-scalar life-2 messages are pruned on Alice's own side too
    // (the inflated version makes them look forgotten by Bob):
    for msg in &life_2[..M] {
        assert!(
            !alice_live.contains_key(msg),
            "Alice's own life-2 message {msg} pruned by her own forget-inference"
        );
        assert!(
            !bob_live.contains_key(msg),
            "Bob never receives Alice's life-2 message {msg} (filtered out before sending)"
        );
    }

    // Above the inflation frontier life-2 propagates as if everything
    // were fine:
    for msg in &life_2[M..] {
        assert_eq!(alice_live.get(msg), Some(&1));
        assert_eq!(bob_live.get(msg), Some(&1));
    }

    // And Carol's foreign-axis message is provably the protection
    // mechanism: it survives on both sides because its subtree's
    // version has a `carol`-component the reused-party attacker cannot
    // dominate.
    assert_eq!(bob_live.get("carol-news"), Some(&1));
    assert_eq!(alice_live.get("carol-news"), Some(&1));

    // Net casualties: `M` legitimate alice-life-1 messages destroyed
    // out of every peer's tree, plus `M` of Alice's life-2 messages
    // destroyed on Alice's own side, plus the protocol still cheerfully
    // reports success and propagates the survivors as if nothing went
    // wrong. The corruption blast radius is `2M` messages from a single
    // reused-party invariant violation.
    assert_eq!(
        bob_live.len(),
        (N - M) + 1,
        "bob's tree contains only the post-inflation life-2 leaves plus carol"
    );
    assert_eq!(
        alice_live.len(),
        (N - M) + 1,
        "alice's tree contains only the post-inflation life-2 leaves plus carol"
    );
}

/// The complementary positive invariant: **strictly causally unrelated data
/// is unbreakable.**
///
/// Define a leaf as *causally unrelated to party P* when its version vector
/// has `version_for(P) == 0` and no transitive ancestor in its causal history
/// ever touched `P`. For such a leaf, no amount of misbehavior by `P`
/// (non-causal `for_party` restarts, concurrent processes sharing P's tag,
/// adversarially-chosen content) can cause the leaf to be destroyed on any
/// peer, regardless of subtree co-location and regardless of the
/// `Unknown::unknown` filter's behavior.
///
/// The structural reason: the filter is `subtree.version <= their_version`
/// *pointwise across every party axis*. The only public API by which a
/// party's view of a foreign axis Q grows is `process` / `gossip`, and both
/// transport the underlying Q-tagged leaves alongside the version-vector
/// update. So `P` cannot construct a version vector that pointwise dominates
/// a subtree whose join carries a Q-axis component without already holding
/// the leaves contributing that component. Consequently the misbehaving
/// party's filter can never strip a subtree whose join has *any* axis the
/// attacker hasn't legitimately advanced.
///
/// This test pins the invariant down empirically:
/// 1. Run the maximum-damage attack from
///    [`scalar_inflation_destroys_unrelated_alice_history`].
/// 2. Have a pristine peer Frank — who never interacted with Alice or her
///    network — publish K messages tagged only with his own party. His
///    leaves' version vectors carry zero on every axis except `frank`.
/// 3. Splice Frank into the corrupted network and gossip exhaustively.
/// 4. Assert that every one of Frank's K leaves is alive on every peer
///    that ever touches him, and that none of his existing leaves is lost
///    on his own side.
#[test]
fn party_misbehavior_cannot_destroy_causally_unrelated_data() {
    const M: usize = 4;
    const N: usize = 8;
    const K: usize = 6;

    // -- Maximum-damage attack (same shape as the previous test) --
    let mut alice = Local::<String, _>::for_party("alice", 0).unwrap();
    let life_1: Vec<String> = (1..=M).map(|i| format!("life-1-msg-{i}")).collect();
    alice.message(life_1.clone(), ignore);

    let mut bob = Local::<String, _>::for_party("bob", 0).unwrap();
    gossip(&mut alice, &mut bob);

    drop(alice);
    let mut alice = Local::<String, _>::for_party("alice", 0).unwrap();
    let life_2: Vec<String> = (1..=N).map(|i| format!("life-2-msg-{i}")).collect();
    alice.message(life_2.clone(), ignore);
    gossip(&mut alice, &mut bob);

    // -- Pristine Frank: no contact with anyone yet. His leaves carry
    //    *only* a `frank`-axis component in their version vectors. --
    let mut frank = Local::<String, _>::for_party("frank", 0).unwrap();
    let frank_msgs: Vec<String> = (1..=K).map(|i| format!("frank-msg-{i}")).collect();
    frank.message(frank_msgs.clone(), ignore);
    assert_eq!(frank.event(), K as u64);

    // -- Splice Frank into the corrupted network. Two ways for the
    //    attack to leak into Frank are exercised: a direct meeting
    //    with the (corrupt) Alice, and a meeting with Bob (whose tree
    //    holds the inflated alice-scalar). Both contagion paths run. --
    gossip(&mut frank, &mut alice);
    gossip(&mut frank, &mut bob);

    // -- All K of Frank's pristine leaves are alive on Frank. --
    let frank_live = live_multiset(&frank);
    for msg in &frank_msgs {
        assert_eq!(
            frank_live.get(msg),
            Some(&1),
            "Frank's own causally-unrelated leaf {msg} survives intact"
        );
    }
    // -- All K of Frank's leaves propagated to Alice and Bob. --
    let alice_live = live_multiset(&alice);
    let bob_live = live_multiset(&bob);
    for msg in &frank_msgs {
        assert_eq!(
            alice_live.get(msg),
            Some(&1),
            "Frank's leaf {msg} propagates to the corrupted Alice unscathed"
        );
        assert_eq!(
            bob_live.get(msg),
            Some(&1),
            "Frank's leaf {msg} propagates to the corrupted Bob unscathed"
        );
    }

    // -- Bring in a third peer Grace who likewise has only her own
    //    pristine messages, and lace her through every peer touched by
    //    the corruption. Her foreign-axis leaves and Frank's are both
    //    preserved everywhere. --
    let mut grace = Local::<String, _>::for_party("grace", 0).unwrap();
    let grace_msgs: Vec<String> = (1..=K).map(|i| format!("grace-msg-{i}")).collect();
    grace.message(grace_msgs.clone(), ignore);
    gossip(&mut grace, &mut bob);
    gossip(&mut grace, &mut frank);
    gossip(&mut grace, &mut alice);

    for peer_live in [
        live_multiset(&alice),
        live_multiset(&bob),
        live_multiset(&frank),
        live_multiset(&grace),
    ] {
        for msg in frank_msgs.iter().chain(grace_msgs.iter()) {
            assert_eq!(
                peer_live.get(msg),
                Some(&1),
                "foreign-axis leaf {msg} survives on every peer after maximum contagion"
            );
        }
    }
}

/// **Even a strictly-correct restart corrupts the network.** The
/// [`Local::for_party`] doc invites the application to persist `event()`
/// and pass it back as `start`, framing that as the discharge of the
/// linearity invariant. But persisting `event()` is *not* enough: it
/// preserves the version-vector scalar without preserving the leaves it
/// dominates, and the resulting peer is a forget-inference bomb pointed
/// at every prior copy of her own messages elsewhere on the network.
///
/// Concretely: Alice's first life originates `M` messages and gossips
/// them to Bob. She crashes. She restarts with `start = M` — the
/// best the documented contract asks for. Her tree is empty, but her
/// version vector immediately equals `(alice, M)`. As soon as she
/// takes *any* event-advancing action (a single insert is enough to
/// push her to `(alice, M+1)`) and then gossips, the
/// [`Unknown::unknown`] filter on Bob's side reads "Alice's view
/// dominates `(alice, k)` for every `k <= M`, yet she lacks the leaves
/// living there" as "Alice has redacted all of her life-1 messages,"
/// and prunes them out of Bob's tree.
///
/// The mechanism is identical to the misbehavior case, but the trigger
/// is canonical "correct" usage. The conclusion is unavoidable: a party
/// identifier must never be reused at all, *not even with the correct
/// `start`*, unless the application also restores the prior life's leaf
/// state into the tree (e.g., by persisting and replaying every message
/// and redaction). Persisting `event()` is a necessary but wildly
/// insufficient condition.
#[test]
fn correct_start_still_destroys_prior_history() {
    const M: usize = 4;

    // -- Alice life-1 publishes M messages and gossips with Bob. --
    let mut alice = Local::<String, _>::for_party("alice", 0).unwrap();
    let life_1: Vec<String> = (1..=M).map(|i| format!("life-1-msg-{i}")).collect();
    alice.message(life_1.clone(), ignore);
    let final_event = alice.event();
    assert_eq!(final_event, M as u64);

    let mut bob = Local::<String, _>::for_party("bob", 0).unwrap();
    gossip(&mut alice, &mut bob);
    for msg in &life_1 {
        assert_eq!(live_multiset(&bob).get(msg), Some(&1));
    }

    // -- Crash. The application *correctly* persisted `event()` and
    //    feeds it back in as `start`, satisfying the documented
    //    contract `start >= event()` with equality. --
    drop(alice);
    let mut alice = Local::<String, _>::for_party("alice", final_event).unwrap();
    assert_eq!(
        alice.event(),
        M as u64,
        "the restarted Alice's view of her own scalar is correctly restored"
    );

    // -- ...but her tree is empty. The persistence advice only
    //    preserved the *scalar*, not the leaves under it. --
    assert!(
        live_multiset(&alice).is_empty(),
        "the persistence recipe gives Alice a scalar but no leaves"
    );

    // -- The application now does anything legitimate. One insert is
    //    enough. Alice's version becomes (alice, M+1). --
    alice.message(["a-fresh-message".to_string()], ignore);
    assert_eq!(alice.event(), (M + 1) as u64);

    // -- Gossip. Bob's filter sees Alice's `(alice, M+1)` dominating
    //    every `(alice, k <= M)` subtree he holds — *every* alice-life-1
    //    leaf in his tree — yet Alice lacks them. Forget-inference
    //    fires, and Bob loses the lot. --
    gossip(&mut alice, &mut bob);

    let alice_live = live_multiset(&alice);
    let bob_live = live_multiset(&bob);
    for msg in &life_1 {
        assert!(
            !bob_live.contains_key(msg),
            "Bob lost legitimate life-1 message {msg} despite Alice restarting with the correct `start`"
        );
        assert!(
            !alice_live.contains_key(msg),
            "Alice never recovers life-1's message {msg} either"
        );
    }
    // The post-restart message survives:
    assert_eq!(alice_live.get("a-fresh-message"), Some(&1));
    assert_eq!(bob_live.get("a-fresh-message"), Some(&1));

    // Net casualties: all M of Alice's faithfully-published prior
    // history, destroyed network-wide by a textbook-correct restart.
}
