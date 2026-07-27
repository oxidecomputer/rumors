//! Golden byte-level snapshots of a single round of gossip between two
//! [`rumors::Rumors`].
//!
//! Each test stages a scenario, drives one gossip session through the
//! recording link in [`common::gossip_snapshot`], and pins every wire byte.
//! V2 frames are grouped by logical stream so nondeterministic cross-stream
//! scheduling does not destabilize the snapshots, while ordering within each
//! stream remains exact. A representative V1 case pins its strictly
//! alternating timeline. Re-accept only after a deliberate protocol change:
//! a new protocol version, never a mutation of an existing one. The
//! re-accept procedure (`cargo insta review`) is in `AGENTS.md`.
//!
//! The payload type is `u64` throughout: it borsh-encodes to a fixed 8 bytes
//! and is trivial to make distinct, which keeps the dumps short and lets
//! distinct payloads (`1`, `2`, `3`, `4`) be spotted directly in the hex.

mod common;

use rand::SeedableRng;
use rand::rngs::SmallRng;
#[cfg(feature = "protocol-v1")]
use rumors::Protocol;
use rumors::{Key, Peer, Rumors};

use crate::common::gossip_snapshot::capture_gossip;
#[cfg(feature = "protocol-v1")]
use crate::common::gossip_snapshot::capture_gossip_v1;
#[cfg(feature = "protocol-v1")]
use crate::common::wire::bootstrap_fork_async_with_protocol;
use crate::common::wire::{block_on, bootstrap_fork, bootstrap_fork_async};

/// A peer seeded from a fixed RNG, so the [`rumors::Network`] id carried in
/// the preamble is deterministic and these byte-level captures stay
/// reproducible across runs.
fn seeded<T>() -> Rumors<T> {
    Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
        .sync_window_floor()
        .into_rumors()
}

/// The key of the live message holding `value`: how a scenario picks out a
/// specific message for redaction. Keys are content-addressed and the
/// scenarios use distinct payloads, so the lookup is unambiguous.
fn key_for(rumors: &Rumors<u64>, value: u64) -> Key {
    rumors
        .snapshot()
        .iter()
        .find_map(|(k, _, m)| (**m == value).then_some(k))
        .unwrap_or_else(|| panic!("no live message holds {value}"))
}

/// Two empty peers: the minimal session.
///
/// After the 25-byte preamble
/// the two sides exchange greetings, find their versions equal, and converge
/// immediately with no content transfer: the protocol's shortest possible
/// conversation.
#[test]
fn empty_pair_converges_immediately() {
    let a: Rumors<u64> = seeded();
    let b = bootstrap_fork(&a);
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// One side holds two messages, the other is an empty peer in the same
/// universe.
///
/// Captures the one-directional flow: the empty peer initiates
/// (the smaller set wins the election) and its empty greeting listing asks
/// for everything, so the populated responder ships its root children as
/// whole height-31 supplies while nothing of substance flows the other way.
#[test]
fn one_sided_transfer() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        // B is a genuine disjoint fork of A, minted while A is still empty, so
        // it is an empty peer in the same universe.
        let b = bootstrap_fork_async(&a).await;

        a.batch().send(1).send(2);
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// Values whose two messages, batch-sent in this order into the seeded
/// universe of [`batched_supply_run`], produce keys sharing their first two
/// bytes (`b4 51`; found by search over the second value).
///
/// The populated
/// responder ships its root children as whole height-31 supplies, so the
/// shared leading byte places both leaves inside one supplied subtree (the
/// two-byte collision is stronger than that supply needs, and keeps the
/// pair inside one subtree at height 30 as well).
const COLLIDING_VALUES: (u64, u64) = (1, 106899);

/// One supplied subtree holding two leaves pins a batched run on the wire.
///
/// Every other fixture supplies single-leaf subtrees, so no other snapshot
/// contains a multi-record run body. Here the transfer's two keys share a
/// two-byte prefix, so the populated peer ships them as a single Supply
/// frame whose run carries two length-prefixed records back to back — the
/// byte-for-byte pin of the batched wire form.
#[test]
fn batched_supply_run() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        let b = bootstrap_fork_async(&a).await;
        let (first, second) = COLLIDING_VALUES;
        a.batch().send(first).send(second);
        (a, b)
    });
    // Self-check the fixture: if hashing or version assignment drifts, fail
    // here with a clear message rather than in the snapshot hex.
    let prefixes: Vec<[u8; 2]> = a
        .snapshot()
        .iter()
        .map(|(k, _, _)| [k.as_bytes()[0], k.as_bytes()[1]])
        .collect();
    assert_eq!(
        prefixes.first(),
        prefixes.last(),
        "the fixture's two keys must share a two-byte prefix to share a supplied subtree"
    );
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// The session honors the smaller of the two exchanged message targets.
///
/// The same two-leaves-one-subtree fixture as [`batched_supply_run`],
/// except the *receiving* (empty) peer declares a zero target. The
/// sender's own default would batch both records into one Supply frame —
/// the previous snapshot pins exactly that — so the two single-record
/// Supply frames pinned here are the receiver's exchanged preference
/// being honored by its peer: the greeting carried it, and the session
/// ran at the minimum of the two settings.
#[test]
fn asymmetric_message_targets_unbatch_the_run() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        let b = bootstrap_fork_async(&a).await;
        let (first, second) = COLLIDING_VALUES;
        a.batch().send(first).send(second);
        let b = b
            .try_into_peer()
            .await
            .expect("the bootstrapped handle is sole")
            .target_message_size(0)
            .into_rumors();
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// Count the frames rendered under one stream header of a wire capture.
///
/// Returns `None` when the header never appears; the header must match the
/// capture's `"{Speaker} stream {index} (height {height})"` form exactly.
fn stream_frames(capture: &str, header: &str) -> Option<Vec<String>> {
    let mut frames = None;
    for line in capture.lines() {
        if line.starts_with(header) {
            frames = Some(Vec::new());
        } else if let Some(frames) = frames.as_mut() {
            match line.trim_start().strip_prefix("frame ") {
                Some(frame) => {
                    let (_, semantic) = frame.split_once(": ").expect("frame lines are labeled");
                    frames.push(semantic.to_string());
                }
                None if line.trim_start().starts_with(char::is_alphabetic) => break,
                None => {}
            }
        }
    }
    frames
}

/// Values whose two messages, batch-sent in this order into the seeded
/// universe of [`bulk_initiator_ships_opening_supplies`], produce keys
/// `b4 51` and `b4 85` (found by search over the second value).
///
/// A shared
/// first byte and distinct second bytes, so the initiator's one exclusive
/// root child holds a two-leaf subtree whose leaves split one level down.
const INITIATOR_SUBTREE_VALUES: (u64, u64) = (1, 148);

/// First of three consecutive ballast values for the responder of
/// [`bulk_initiator_ships_opening_supplies`].
///
/// Their keys' first bytes
/// (`9f`, `e3`, `e5`) avoid the initiator's exclusive radix (`b4`), and the
/// extra message makes the responder the larger set, so the subtree holder
/// wins the initiator election.
const RESPONDER_BALLAST_FROM: u64 = 100;

/// A bulk-holding initiator ships its exclusive root children whole at the
/// opening, on its own stream 0, without waiting for the responder's empty
/// queries.
///
/// The initiator (the smaller set) holds one exclusive root child with two
/// leaves splitting at the second key byte. The pinned shape is the
/// supply-only opening: the whole child crosses as a single two-record
/// Supply run on `Initiator stream 0 (height 31)`, and the responder's
/// root-level empty query is answered by a bare empty reply at height 30
/// instead of one decomposed Supply frame per second-byte child.
#[test]
fn bulk_initiator_ships_opening_supplies() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        let b = bootstrap_fork_async(&a).await;
        let (first, second) = INITIATOR_SUBTREE_VALUES;
        a.batch().send(first).send(second);
        let y = RESPONDER_BALLAST_FROM;
        b.batch().send(y).send(y + 1).send(y + 2);
        (a, b)
    });

    // Fixture self-checks: the initiator-exclusive subtree and the election.
    let akeys: Vec<[u8; 2]> = a
        .snapshot()
        .iter()
        .map(|(k, _, _)| [k.as_bytes()[0], k.as_bytes()[1]])
        .collect();
    assert_eq!(
        akeys.first().map(|k| k[0]),
        akeys.last().map(|k| k[0]),
        "the initiator's two keys must share a root radix"
    );
    assert_ne!(
        akeys.first().map(|k| k[1]),
        akeys.last().map(|k| k[1]),
        "the initiator's two keys must split one level below the root"
    );
    let radix = akeys[0][0];
    assert!(
        b.snapshot()
            .iter()
            .all(|(k, _, _)| k.as_bytes()[0] != radix),
        "the responder must lack the initiator's exclusive radix"
    );
    assert!(
        a.snapshot().len() < b.snapshot().len(),
        "the subtree holder must advertise the smaller set and initiate"
    );

    let capture = capture_gossip(a, b);
    let opening = stream_frames(&capture, "Initiator stream 0 (height 31)")
        .expect("the initiator's opening supplies ride its stream 0");
    assert_eq!(
        opening,
        ["Supply(End)", "End(Stream)"],
        "the exclusive subtree crosses whole: one batched Supply run"
    );
    let height_30 = stream_frames(&capture, "Initiator stream 1 (height 30)")
        .expect("the responder's root-level empty query still gets its reply");
    assert_eq!(
        height_30,
        ["End(Reply)", "End(Stream)"],
        "the empty query's answer is an empty reply: the content crossed at \
         the opening"
    );
    insta::assert_snapshot!(capture);
}

/// Values for [`early_supplies_honor_redactions`]: the second, sent after
/// the responder forks, lands its key under the same root radix as the
/// first's (keys `8a 44` and `8a fd`), found by search.
const REDACTION_SUBTREE_VALUE: u64 = 33;

/// First of three consecutive ballast values for the responder of
/// [`early_supplies_honor_redactions`]: their keys' first bytes (`70`,
/// `c8`, `eb`) avoid the shared radix (`8a`), and they make the responder
/// the larger set.
const REDACTION_BALLAST_FROM: u64 = 100;

/// Deletion honoring prunes the opening supplies: a redacted message does
/// not resurrect through the early path, and the supply carries the
/// survivor rather than the full subtree.
///
/// The initiator holds two messages under one root radix; the responder
/// once held the first (it forked after it existed), redacted it, and holds
/// ballast elsewhere. The initiator's early supply for that radix must
/// carry only the second message — the survivor of pruning against the
/// responder's version — and both peers converge with the redacted message
/// gone. Two records in the supply run would resurrect the redaction; the
/// pinned bytes show one.
#[test]
fn early_supplies_honor_redactions() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        a.send(1);
        let b = bootstrap_fork_async(&a).await;
        a.send(REDACTION_SUBTREE_VALUE);
        b.redact(key_for(&b, 1));
        let y = REDACTION_BALLAST_FROM;
        b.batch().send(y).send(y + 1).send(y + 2);
        (a, b)
    });

    // Fixture self-checks: shared radix, cover of the redacted message,
    // and the election.
    let akeys: Vec<u8> = a
        .snapshot()
        .iter()
        .map(|(k, _, _)| k.as_bytes()[0])
        .collect();
    assert_eq!(akeys.len(), 2, "the initiator holds the pair");
    assert_eq!(
        akeys.first(),
        akeys.last(),
        "both initiator keys must share a root radix"
    );
    let radix = akeys[0];
    assert!(
        b.snapshot()
            .iter()
            .all(|(k, _, _)| k.as_bytes()[0] != radix),
        "the responder must lack the shared radix outright: it redacted \
         its copy"
    );
    assert!(
        a.snapshot().len() < b.snapshot().len(),
        "the subtree holder must advertise the smaller set and initiate"
    );

    let capture = capture_gossip(a.clone(), b.clone());
    let opening = stream_frames(&capture, "Initiator stream 0 (height 31)")
        .expect("the surviving message rides the opening supplies");
    assert_eq!(
        opening,
        ["Supply(End)", "End(Stream)"],
        "one pruned Supply run: the survivor, not the full subtree"
    );
    assert!(
        !a.snapshot().iter().any(|(_, _, m)| **m == 1),
        "the redaction is contagious: the initiator drops the message"
    );
    assert!(
        !b.snapshot().iter().any(|(_, _, m)| **m == 1),
        "the redacted message must not resurrect at the responder"
    );
    assert!(
        a.snapshot()
            .iter()
            .any(|(_, _, m)| **m == REDACTION_SUBTREE_VALUE),
        "the survivor converges to the initiator"
    );
    assert!(
        b.snapshot()
            .iter()
            .any(|(_, _, m)| **m == REDACTION_SUBTREE_VALUE),
        "the survivor converges to the responder"
    );
    insta::assert_snapshot!(capture);
}

/// V1 retains its original strict alternating transcript through the public
/// selector, including content transfer rather than only an empty handshake.
#[cfg(feature = "protocol-v1")]
#[test]
fn v1_one_sided_transfer() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
            .sync_window_floor()
            .protocol(Protocol::V1)
            .into_rumors();
        let b = bootstrap_fork_async_with_protocol(&a, Protocol::V1).await;
        a.batch().send(1).send(2);
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip_v1(a, b));
}

/// The headline scenario, exercising most of the wire protocol's properties in
/// one session:
///
/// 1. seed `A` and insert two distinct messages (`1`, `2`);
/// 2. fork `B` from `A` (both now hold `1` and `2`, sharing their keys);
/// 3. each fork inserts one distinct message (`A` adds `3`, `B` adds `4`);
/// 4. each fork redacts a *different* one of the two common messages (`A`
///    redacts `1`, `B` redacts `2`);
/// 5. gossip.
///
/// Reconciliation must converge both peers on the live set `{3, 4}`: the two
/// redactions are contagious and cross the wire alongside the two novel
/// inserts, so the capture pins inserts, fork divergence, bidirectional
/// transfer, and redaction propagation all at once.
#[test]
fn fork_insert_redact() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();

        // (1) Two distinct common messages.
        a.batch().send(1).send(2);

        // (2) Fork: B is a genuine disjoint fork sharing A's observations
        // (both hold 1 and 2, under the same keys).
        let b = bootstrap_fork_async(&a).await;

        // (3) Each fork inserts one distinct message.
        a.send(3);
        b.send(4);

        // (4) Each fork redacts a different one of the two common messages.
        a.redact(key_for(&a, 1));
        b.redact(key_for(&b, 2));

        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// Fork with *no* divergence: insert `1` and `2`, fork, gossip immediately.
///
/// Both peers carry identical content *and* identical version vectors, so the
/// version exchange short-circuits the session to Done before any content is
/// examined — zero transfer despite non-empty trees. The non-empty companion
/// to [`empty_pair_converges_immediately`]: it proves convergence is decided
/// by version equality, independent of how much content the peers hold.
#[test]
fn converged_forks_noop() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        a.batch().send(1).send(2);
        let b = bootstrap_fork_async(&a).await;
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// Redaction in isolation: both forks hold `1` and `2`, `A` redacts `1`, `B`
/// does nothing, then they gossip and converge on `{2}`.
///
/// The clean counterpart
/// to [`fork_insert_redact`] — no inserts share the wire, so the bytes that
/// carry a redaction (and the version advance that distinguishes "forgot it"
/// from "never had it") stand alone.
#[test]
fn redaction_only() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        a.batch().send(1).send(2);
        let b = bootstrap_fork_async(&a).await;
        a.redact(key_for(&a, 1));
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// Number of disjoint messages each side of [`deep_trie_divergence`] holds.
///
/// Chosen so the two sides' leaves are numerous enough to collide in their
/// leading hash byte, branching the trie past its root and so driving the
/// recursive `Exchange` descent (and the `Opening`/`Closing`/`Complete` phases
/// at more than one level) that the small scenarios never reach.
const DEEP_TRIE_PER_SIDE: u64 = 16;

/// Two peers with large, fully disjoint message sets.
///
/// `A` holds
/// `0..DEEP_TRIE_PER_SIDE`, `B` holds the next `DEEP_TRIE_PER_SIDE`; both
/// descend from one seed so they may gossip, but they share no content. The
/// reconciliation must branch the prefix-trie and recurse down it, exercising
/// the protocol's recursive core that the handful-of-messages scenarios leave
/// untouched.
#[test]
fn deep_trie_divergence() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        let b = bootstrap_fork_async(&a).await;
        {
            let mut batch = a.batch();
            for v in 0..DEEP_TRIE_PER_SIDE {
                batch.send(v);
            }
        }
        {
            let mut batch = b.batch();
            for v in DEEP_TRIE_PER_SIDE..2 * DEEP_TRIE_PER_SIDE {
                batch.send(v);
            }
        }
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// A non-primitive, variable-length payload type.
///
/// `u64` borsh-encodes to a
/// fixed 8 bytes; `String` encodes as a length prefix followed by its UTF-8
/// bytes, so this is the only scenario that pins how a variable-length value
/// is framed inside a leaf on the wire. `A` and `B` each contribute one
/// distinct string and converge on both.
#[test]
fn string_payload() {
    let (a, b) = block_on(async {
        let a: Rumors<String> = seeded();
        let b = bootstrap_fork_async(&a).await;
        a.send("hello".to_string());
        b.send("world".to_string());
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// Equal live content, divergent version vectors.
///
/// Both peers hold `1`; then
/// `A` inserts `2` and immediately redacts it, leaving its *live* set back at
/// `{1}` but advancing its version vector past `B`'s. The two peers' observable
/// root hashes are therefore equal while their versions are not — so this pins
/// whether the protocol short-circuits on the matching live hash or whether the
/// version dominance (the same signal redaction propagation rides on) drives a
/// reconciliation pass. There are no deletion markers in the protocol; the only
/// trace of the redacted `2` is the advanced version.
#[test]
fn same_live_content_divergent_versions() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        a.send(1);
        let b = bootstrap_fork_async(&a).await;

        // A diverges in version but not in live content: insert 2, then drop it.
        a.send(2);
        a.redact(key_for(&a, 2));
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// Concurrent, identical redaction.
///
/// Both forks hold `1` and `2`, and *each*
/// independently redacts `1` (the same [`Key`]) before they gossip. The two
/// redactions are causally concurrent — distinct version advances on distinct
/// parties — yet target the same message, so this pins that the protocol
/// converges idempotently on `{2}` rather than treating the two redactions as
/// conflicting work to reconcile.
#[test]
fn both_redact_same_key() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        a.batch().send(1).send(2);
        let b = bootstrap_fork_async(&a).await;
        let k1 = key_for(&a, 1);
        a.redact(k1);
        b.redact(k1);
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}
