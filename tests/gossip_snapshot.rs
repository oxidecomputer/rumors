//! Golden byte-level snapshots of a single round of gossip between two
//! [`rumors::Rumors`].
//!
//! Each test stages a scenario, drives one gossip session through the
//! recording duplex in [`common::gossip_snapshot`], and pins every wire byte.
//! V2 frames are grouped by logical stream so nondeterministic cross-stream
//! scheduling does not destabilize the snapshots, while ordering within each
//! stream remains exact. A representative V1 case pins its strictly
//! alternating timeline. Re-accept only after a deliberate protocol change.
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
    Peer::seed_rng(&mut SmallRng::seed_from_u64(0)).into_rumors()
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

/// Two empty peers: the minimal session. After the 25-byte preamble
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
/// universe. Captures the one-directional flow: the populated peer ships its
/// content and the empty peer requests and receives it, with nothing of
/// substance flowing the other way.
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

/// V1 retains its original strict alternating transcript through the public
/// selector, including content transfer rather than only an empty handshake.
#[cfg(feature = "protocol-v1")]
#[test]
fn v1_one_sided_transfer() {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
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
/// does nothing, then they gossip and converge on `{2}`. The clean counterpart
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
/// Chosen so the two sides' leaves are numerous enough to collide in their
/// leading hash byte, branching the trie past its root and so driving the
/// recursive `Exchange` descent (and the `Opening`/`Closing`/`Complete` phases
/// at more than one level) that the small scenarios never reach.
const DEEP_TRIE_PER_SIDE: u64 = 16;

/// Two peers with large, fully disjoint message sets. `A` holds
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

/// A non-primitive, variable-length payload type. `u64` borsh-encodes to a
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

/// Equal live content, divergent version vectors. Both peers hold `1`; then
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

/// Concurrent, identical redaction. Both forks hold `1` and `2`, and *each*
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
