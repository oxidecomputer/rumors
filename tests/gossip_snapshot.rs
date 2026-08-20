//! Golden byte-level snapshots of a single round of gossip between two
//! [`rumors::Rumors`].
//!
//! Each test stages a scenario, drives one gossip session through the
//! recording link in [`common::gossip_snapshot`], and pins every wire byte.
//! V2 frames are grouped by logical stream so nondeterministic cross-stream
//! scheduling does not destabilize the snapshots, while ordering within each
//! stream remains exact. A representative V1 case pins its strictly
//! alternating timeline. Re-accept only after a deliberate protocol change,
//! never as an accommodation of drift; the two-regime re-accept rule and
//! its procedure (`cargo insta review`) are in `AGENTS.md`.
//!
//! The payload type is `u64` throughout: a small integer is one CBOR byte
//! (`01`, `02`, …), which keeps the dumps short and lets distinct payloads
//! be spotted directly in the hex.

mod common;

use rand::SeedableRng;
use rand::rngs::SmallRng;
#[cfg(feature = "protocol-v1")]
use rumors::Protocol;
use rumors::{Peer, Rumors, Version};

#[cfg(feature = "protocol-v1")]
use crate::common::gossip_snapshot::capture_gossip_v1;
use crate::common::gossip_snapshot::{capture_gossip, capture_gossip_returning};
use crate::common::shape::{
    ballast_avoiding, keep_only, leaf_path, path_radix, pool, send_pool, shaped_pair,
};
#[cfg(feature = "protocol-v1")]
use crate::common::wire::bootstrap_fork_async_with_protocol;
use crate::common::wire::{batch_send, block_on, bootstrap_fork, bootstrap_fork_async};

/// A peer seeded from a fixed RNG, so the [`rumors::Network`] id carried in
/// the preamble is deterministic and these byte-level captures stay
/// reproducible across runs.
fn seeded<T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static>()
-> Rumors<T> {
    Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
        .sync_window_floor()
        .into_rumors()
}

/// The version of the live message holding `value`: how a scenario picks
/// out a specific message for redaction. The scenarios use distinct
/// payloads, so the lookup is unambiguous.
fn version_for(rumors: &Rumors<u64>, value: u64) -> Version {
    rumors
        .snapshot()
        .iter()
        .find_map(|(v, m)| (*m == value).then_some(v.clone()))
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

        batch_send(&a, [1, 2]);
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// Pool size for [`colliding_pair`]'s two-byte search: paths are uniform,
/// so a two-byte agreement needs a birthday-scale pool over 2¹⁶.
const COLLIDING_POOL: u64 = 1024;

/// Stage the batched-run universe: a populated peer holding exactly two
/// leaves whose paths share their first two bytes, against an empty fork.
///
/// Paths are version-derived, so the shape is staged by minting a pool of
/// sends, searching the minted versions for the first two-byte agreement,
/// and redacting the rest — deterministic under the seeded universe (see
/// `common::shape`). The shared leading byte places both leaves inside one
/// supplied root child (the two-byte agreement is stronger than that
/// supply needs, and keeps the pair inside one subtree at height 30 as
/// well).
fn colliding_pair() -> (Rumors<u64>, Rumors<u64>) {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        let b = bootstrap_fork_async(&a).await;
        send_pool(&a, 0, COLLIDING_POOL);
        (a, b)
    });
    let (first, second) = shaped_pair(&pool(&a, 0, COLLIDING_POOL), 2, false);
    keep_only(&a, 0, COLLIDING_POOL, &[first, second]);

    // Self-check the landed shape: if hashing or version assignment
    // drifts, fail here with a clear message rather than in the hex.
    let prefixes: Vec<[u8; 2]> = a
        .snapshot()
        .iter()
        .map(|(v, _)| {
            let path = leaf_path(v);
            [path[0], path[1]]
        })
        .collect();
    assert_eq!(prefixes.len(), 2, "the fixture holds exactly the pair");
    assert_eq!(
        prefixes.first(),
        prefixes.last(),
        "the fixture's two leaf paths must share a two-byte prefix to share a supplied subtree"
    );
    (a, b)
}

/// One supplied subtree holding two leaves pins a batched run on the wire.
///
/// Every other fixture supplies single-leaf subtrees, so no other snapshot
/// contains a multi-record run body. Here the transfer's two leaf paths
/// share a two-byte prefix, so the populated peer ships them as a single
/// Supply frame whose run carries two length-prefixed records back to back
/// — the byte-for-byte pin of the batched wire form.
#[test]
fn batched_supply_run() {
    let (a, b) = colliding_pair();
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
    let (a, b) = colliding_pair();
    let b = block_on(async {
        b.try_into_peer()
            .await
            .expect("the bootstrapped handle is sole")
            .target_message_size(0)
            .into_rumors()
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// Extract one rendered signal line's semantic.
///
/// A signal line has the form `<dense code> / <semantic> /`. The
/// bare-digit code distinguishes signal lines from every other
/// annotated line (tagged atoms carry parentheses, listings carry
/// `=>`, payloads carry no comment), so the extraction cannot misfire
/// inside a frame body.
fn signal_semantic(line: &str) -> Option<&str> {
    let (code, rest) = line.trim_start().split_once(" / ")?;
    if code.is_empty() || !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.strip_suffix(" /")
}

/// The child count of every nonempty-Query frame body in a capture.
///
/// A `Query(…)` signal line is followed by its frame's listing body,
/// which opens with `{ / listing: <n> child(ren) /`. Greeting listings
/// render the same annotation, so the scan keys on the Query signal
/// and reads only until the next signal or column-zero header.
fn nonempty_query_listings(capture: &str) -> Vec<usize> {
    let mut counts = Vec::new();
    let mut in_query = false;
    for line in capture.lines() {
        if let Some(semantic) = signal_semantic(line) {
            in_query = semantic.starts_with("Query(");
        } else if !line.starts_with(char::is_whitespace) {
            in_query = false;
        } else if in_query
            && let Some(rest) = line.trim_start().strip_prefix("{ / listing: ")
            && let Some(n) = rest.split_whitespace().next().and_then(|n| n.parse().ok())
        {
            counts.push(n);
            in_query = false;
        }
    }
    counts
}

/// Count the frames rendered under one stream header of a wire capture.
///
/// Returns `None` when the header never appears; the header must be a
/// prefix of the capture's
/// `"{Speaker} stream {index} (height {height}), epoch {e}, {n} wire bytes"`
/// header line. A stream's body lines — frame headers, rendered value
/// trees — are all indented, so the section ends at the next column-zero
/// line (the following stream or direction header, or a control-item
/// label); within the section each frame contributes exactly one signal
/// line, whose comment is the frame's semantic.
fn stream_frames(capture: &str, header: &str) -> Option<Vec<String>> {
    let mut frames = None;
    for line in capture.lines() {
        if line.starts_with(header) {
            frames = Some(Vec::new());
        } else if let Some(frames) = frames.as_mut() {
            if let Some(semantic) = signal_semantic(line) {
                frames.push(semantic.to_string());
            } else if !line.starts_with(char::is_whitespace) {
                break;
            }
        }
    }
    frames
}

/// Pool size for a one-byte *pair* search (any two paths agreeing on
/// their root radix): a birthday search over 256 radixes, hit early.
const RADIX_POOL: u64 = 64;

/// Pool size for hitting one *specific* root radix: a direct-hit search
/// with mean 256, sized well past it.
const TARGETED_POOL: u64 = 2048;

/// Payload base and pool size for a fixture's responder ballast: a
/// disjoint payload range so pool cleanups never touch the other side's
/// messages.
const BALLAST_POOL: (u64, u64) = (10_000, 16);

/// A bulk-holding initiator ships its exclusive root children whole at the
/// opening, on its own stream 0, without waiting for the responder's empty
/// queries.
///
/// The initiator (the smaller set) holds one exclusive root child with two
/// leaves splitting at the second path byte. The pinned shape is the
/// supply-only opening: the whole child crosses as a single two-record
/// Supply run on `Initiator stream 0 (height 31)`, and the responder's
/// root-level empty query is answered by a bare empty reply at height 30
/// instead of one decomposed Supply frame per second-byte child.
#[test]
fn bulk_initiator_ships_opening_supplies() {
    // Stage: the initiator holds exactly two leaves sharing a root radix
    // and splitting one level down (pool-search-and-redact; the shape is a
    // function of the minted versions, see `common::shape`); the responder
    // holds three ballast leaves outside that radix, making it the larger
    // set so the subtree holder initiates.
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        let b = bootstrap_fork_async(&a).await;
        send_pool(&a, 0, RADIX_POOL);
        (a, b)
    });
    let (first, second) = shaped_pair(&pool(&a, 0, RADIX_POOL), 1, true);
    keep_only(&a, 0, RADIX_POOL, &[first, second]);
    let radix = path_radix(&version_for(&a, first));
    let (ballast_from, ballast_pool) = BALLAST_POOL;
    send_pool(&b, ballast_from, ballast_pool);
    let ballast = ballast_avoiding(&pool(&b, ballast_from, ballast_pool), radix, 3);
    keep_only(&b, ballast_from, ballast_pool, &ballast);

    // Fixture self-checks: the initiator-exclusive subtree and the election.
    let apaths: Vec<[u8; 2]> = a
        .snapshot()
        .iter()
        .map(|(v, _)| {
            let path = leaf_path(v);
            [path[0], path[1]]
        })
        .collect();
    assert_eq!(
        apaths.first().map(|p| p[0]),
        apaths.last().map(|p| p[0]),
        "the initiator's two leaf paths must share a root radix"
    );
    assert_ne!(
        apaths.first().map(|p| p[1]),
        apaths.last().map(|p| p[1]),
        "the initiator's two leaf paths must split one level below the root"
    );
    let radix = apaths[0][0];
    assert!(
        b.snapshot().iter().all(|(v, _)| leaf_path(v)[0] != radix),
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
    // Stage: the initiator's first message exists before the fork (so the
    // responder once held it), and a pool search lands a second initiator
    // leaf under the same root radix; the responder redacts its copy of
    // the first and keeps three ballast leaves outside that radix, making
    // it the larger set.
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        a.send(1).unwrap();
        let b = bootstrap_fork_async(&a).await;
        b.redact(&version_for(&b, 1));
        (a, b)
    });
    let radix = path_radix(&version_for(&a, 1));
    send_pool(&a, 2, TARGETED_POOL);
    let sibling = pool(&a, 2, TARGETED_POOL)
        .into_iter()
        .find(|(_, v)| path_radix(v) == radix)
        .map(|(value, _)| value)
        .expect("some pool leaf lands under the first message's radix");
    keep_only(&a, 2, TARGETED_POOL, &[1, sibling]);
    let (ballast_from, ballast_pool) = BALLAST_POOL;
    send_pool(&b, ballast_from, ballast_pool);
    let ballast = ballast_avoiding(&pool(&b, ballast_from, ballast_pool), radix, 3);
    keep_only(&b, ballast_from, ballast_pool, &ballast);

    // Fixture self-checks: shared radix, cover of the redacted message,
    // and the election.
    let apaths: Vec<u8> = a.snapshot().iter().map(|(v, _)| leaf_path(v)[0]).collect();
    assert_eq!(apaths.len(), 2, "the initiator holds the pair");
    assert_eq!(
        apaths.first(),
        apaths.last(),
        "both initiator leaf paths must share a root radix"
    );
    let radix = apaths[0];
    assert!(
        b.snapshot().iter().all(|(v, _)| leaf_path(v)[0] != radix),
        "the responder must lack the shared radix outright: it redacted \
         its copy"
    );
    assert!(
        a.snapshot().len() < b.snapshot().len(),
        "the subtree holder must advertise the smaller set and initiate"
    );

    let (capture, a, b) = capture_gossip_returning(a, b);
    let opening = stream_frames(&capture, "Initiator stream 0 (height 31)")
        .expect("the surviving message rides the opening supplies");
    assert_eq!(
        opening,
        ["Supply(End)", "End(Stream)"],
        "one pruned Supply run: the survivor, not the full subtree"
    );
    assert!(
        !a.snapshot().iter().any(|(_, m)| *m == 1),
        "the redaction is contagious: the initiator drops the message"
    );
    assert!(
        !b.snapshot().iter().any(|(_, m)| *m == 1),
        "the redacted message must not resurrect at the responder"
    );
    assert!(
        a.snapshot().iter().any(|(_, m)| *m == sibling),
        "the survivor converges to the initiator"
    );
    assert!(
        b.snapshot().iter().any(|(_, m)| *m == sibling),
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
        batch_send(&a, [1, 2]);
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
        batch_send(&a, [1, 2]);

        // (2) Fork: B is a genuine disjoint fork sharing A's observations
        // (both hold 1 and 2, under the same versions).
        let b = bootstrap_fork_async(&a).await;

        // (3) Each fork inserts one distinct message.
        a.send(3).unwrap();
        b.send(4).unwrap();

        // (4) Each fork redacts a different one of the two common messages.
        a.redact(&version_for(&a, 1));
        b.redact(&version_for(&b, 2));

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
        batch_send(&a, [1, 2]);
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
        batch_send(&a, [1, 2]);
        let b = bootstrap_fork_async(&a).await;
        a.redact(&version_for(&a, 1));
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
            a.batch(|batch| {
                for v in 0..DEEP_TRIE_PER_SIDE {
                    batch.send(v)?;
                }
                Ok::<(), rumors::EncodeError>(())
            })
            .expect("flat test payloads are within any depth limit");
        }
        {
            b.batch(|batch| {
                for v in DEEP_TRIE_PER_SIDE..2 * DEEP_TRIE_PER_SIDE {
                    batch.send(v)?;
                }
                Ok::<(), rumors::EncodeError>(())
            })
            .expect("flat test payloads are within any depth limit");
        }
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// Payload base for [`shared_subtree_dispute_pins_a_nonempty_query`]'s
/// divergence pool: disjoint from the fixture's first pool so the
/// second cleanup never touches the shared pair.
const DISPUTE_POOL_BASE: u64 = 100;

/// A disputed shared subtree pins the nonempty Query frame: the one
/// wire form no other fixture provokes.
///
/// The other fixtures' nonempty listings ride only inside greetings,
/// and every wire query they pin is `QueryEmpty`. Here both peers hold
/// the same two leaves under one root radix, splitting at the second
/// path byte, so the shared node is a genuine two-child branch on each
/// side; the populated side then adds a third leaf under the same
/// radix. The two sides' subtree hashes differ while neither side is
/// absent (an absent side yields supplies) and the subtrees are not
/// identical (identical subtrees yield matches), so answering the
/// dispute *lists children*: a Query frame carrying a nonempty
/// `{radix => digest}` listing. The in-test liveness floor asserts
/// that listing before the snapshot comparison, so the fixture cannot
/// silently degrade back to `QueryEmpty` under a future corpus change.
#[test]
fn shared_subtree_dispute_pins_a_nonempty_query() {
    // Stage: two leaves sharing a root radix and splitting at the
    // second byte (pool-search-and-redact, see `common::shape`),
    // staged before the fork so both peers hold the branch
    // identically.
    let a: Rumors<u64> = seeded();
    send_pool(&a, 0, RADIX_POOL);
    let (first, second) = shaped_pair(&pool(&a, 0, RADIX_POOL), 1, true);
    keep_only(&a, 0, RADIX_POOL, &[first, second]);
    let b = bootstrap_fork(&a);

    // Diverge under the shared radix: a third leaf lands there on the
    // populated side alone.
    let radix = path_radix(&version_for(&a, first));
    send_pool(&a, DISPUTE_POOL_BASE, TARGETED_POOL);
    let third = pool(&a, DISPUTE_POOL_BASE, TARGETED_POOL)
        .into_iter()
        .find(|(_, v)| path_radix(v) == radix)
        .map(|(value, _)| value)
        .expect("some pool leaf lands under the shared radix");
    keep_only(&a, DISPUTE_POOL_BASE, TARGETED_POOL, &[third]);

    // Fixture self-checks: the two-child shared branch, the
    // divergence, and the election.
    let paths = |rumors: &Rumors<u64>| -> Vec<[u8; 2]> {
        rumors
            .snapshot()
            .iter()
            .map(|(v, _)| {
                let path = leaf_path(v);
                [path[0], path[1]]
            })
            .collect()
    };
    let apaths = paths(&a);
    assert_eq!(
        apaths.len(),
        3,
        "the populated side holds the pair plus the divergent leaf"
    );
    assert!(
        apaths.iter().all(|p| p[0] == radix),
        "every leaf sits under the one shared root radix"
    );
    let bpaths = paths(&b);
    assert_eq!(bpaths.len(), 2, "the fork holds exactly the shared pair");
    assert!(
        bpaths.iter().all(|p| p[0] == radix),
        "the fork's leaves sit under the same shared radix"
    );
    assert_ne!(
        bpaths[0][1], bpaths[1][1],
        "the shared subtree branches at the second byte: a genuine \
         two-child node on both sides"
    );
    assert!(
        b.snapshot().len() < a.snapshot().len(),
        "the fork advertises the smaller set and initiates"
    );

    let capture = capture_gossip(a, b);
    // The pin's liveness floor, asserted before the snapshot
    // comparison: at least one Query frame carries a nonempty child
    // listing.
    let listings = nonempty_query_listings(&capture);
    assert!(
        listings.iter().any(|&n| n >= 1),
        "the dispute must pin a nonempty Query frame, not degrade to \
         QueryEmpty; capture:\n{capture}"
    );
    insta::assert_snapshot!(capture);
}

/// A non-primitive, variable-length payload type.
///
/// A `String` encodes as a CBOR text string — a header byte carrying the
/// length, then the UTF-8 bytes — so this is the scenario that pins how a
/// variable-length value is framed inside a leaf on the wire. `A` and `B`
/// each contribute one distinct string and converge on both.
#[test]
fn string_payload() {
    let (a, b) = block_on(async {
        let a: Rumors<String> = seeded();
        let b = bootstrap_fork_async(&a).await;
        a.send("hello".to_string()).unwrap();
        b.send("world".to_string()).unwrap();
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
        a.send(1).unwrap();
        let b = bootstrap_fork_async(&a).await;

        // A diverges in version but not in live content: insert 2, then drop it.
        a.send(2).unwrap();
        a.redact(&version_for(&a, 2));
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}

/// Concurrent, identical redaction.
///
/// Both forks hold `1` and `2`, and *each*
/// independently redacts `1` (the same message: a bootstrap copies the
/// leaf, [`Version`] included) before they gossip. The two redactions are
/// causally concurrent — distinct version advances on distinct parties —
/// yet target the same message, so this pins that the protocol converges
/// idempotently on `{2}` rather than treating the two redactions as
/// conflicting work to reconcile.
#[test]
fn both_redact_the_same_message() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = seeded();
        batch_send(&a, [1, 2]);
        let b = bootstrap_fork_async(&a).await;
        let v1 = version_for(&a, 1);
        a.redact(&v1);
        b.redact(&v1);
        (a, b)
    });
    insta::assert_snapshot!(capture_gossip(a, b));
}
