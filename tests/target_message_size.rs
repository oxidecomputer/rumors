//! End-to-end coverage of [`rumors::Peer::target_message_size`]: the knob
//! threads from the public builder into the streaming session, any target
//! (including the degenerate zero) leaves reconciliation convergent, and
//! peers with different targets interoperate because run sizing is not
//! wire-visible.

mod common;

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::{DEFAULT_TARGET_MESSAGE_SIZE, Peer, Rumors};

use crate::common::gossip_snapshot::capture_gossip;
use crate::common::wire::{block_on, bootstrap_fork_async, wire_gossip_async};

/// Messages each side originates after the fork: enough for multi-leaf
/// supplied subtrees, so run batching is genuinely exercised on the wire.
const DIVERGENT_PER_SIDE: usize = 96;

/// A divergent pair whose sides select different run targets before any
/// session runs, so both encoders exercise their own setting.
fn diverged_pair(left_target: usize, right_target: usize) -> (Rumors<u64>, Rumors<u64>) {
    block_on(async {
        let left = Peer::seed().target_message_size(left_target).into_rumors();
        let right = bootstrap_fork_async(&left).await;
        let right = right
            .try_into_peer()
            .await
            .expect("the fresh fork has a single handle")
            .target_message_size(right_target)
            .into_rumors();

        let mut rng = SmallRng::seed_from_u64(0x5eed_0f1e_a55e_d000);
        for _ in 0..DIVERGENT_PER_SIDE {
            left.send(rng.next_u64());
            right.send(rng.next_u64());
        }
        (left, right)
    })
}

/// Assert one gossip session converges the pair to identical live sets.
fn assert_converges(pair: (Rumors<u64>, Rumors<u64>)) {
    let (left, right) = pair;
    block_on(wire_gossip_async(&left, &right));
    let (left, right) = (left.snapshot(), right.snapshot());
    assert_eq!(left.len(), right.len());
    assert_eq!(
        left.iter().map(|(k, _, _)| k).collect::<Vec<_>>(),
        right.iter().map(|(k, _, _)| k).collect::<Vec<_>>(),
    );
}

/// A zero target — every leaf in its own run, the pre-batching wire
/// traffic — still reconciles a divergent pair completely.
#[test]
fn zero_target_still_converges() {
    assert_converges(diverged_pair(0, 0));
}

/// Peers with different targets interoperate: run sizing is a local
/// encoder choice, not a negotiated wire parameter.
#[test]
fn mixed_targets_interoperate() {
    assert_converges(diverged_pair(0, DEFAULT_TARGET_MESSAGE_SIZE));
}

/// Like [`diverged_pair`], but with deterministically seeded peer identities,
/// so the same divergence replays byte-identically across sessions and their
/// wire captures are comparable.
fn seeded_diverged_pair(target: usize) -> (Rumors<u64>, Rumors<u64>) {
    block_on(async {
        let left = Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
            .target_message_size(target)
            .into_rumors();
        let right = bootstrap_fork_async(&left).await;
        let right = right
            .try_into_peer()
            .await
            .expect("the fresh fork has a single handle")
            .target_message_size(target)
            .into_rumors();

        let mut rng = SmallRng::seed_from_u64(0x5eed_0f1e_a55e_d000);
        for _ in 0..DIVERGENT_PER_SIDE {
            left.send(rng.next_u64());
            right.send(rng.next_u64());
        }
        (left, right)
    })
}

/// Count the supply frames (both directions) in a rendered wire capture.
fn supply_frames(capture: &str) -> usize {
    capture.matches(": Supply").count()
}

/// The knob genuinely reaches the wire, which convergence alone cannot show.
///
/// The same seeded divergence is reconciled twice, its wire traffic captured
/// through the recording link: a zero target forbids batching (every leaf
/// rides its own supply frame), while the default target batches each
/// supplied subtree's leaves into runs — so the zero-target session must
/// emit strictly more supply frames. If the budget silently regressed to a
/// single behavior, the two counts would be equal and this fails.
#[test]
fn zero_target_emits_more_supply_frames_than_default() {
    let zero = {
        let (left, right) = seeded_diverged_pair(0);
        capture_gossip(left, right)
    };
    let batched = {
        let (left, right) = seeded_diverged_pair(DEFAULT_TARGET_MESSAGE_SIZE);
        capture_gossip(left, right)
    };
    let (zero_frames, batched_frames) = (supply_frames(&zero), supply_frames(&batched));
    assert!(
        batched_frames > 0,
        "a divergent session must supply at least one run"
    );
    assert!(
        zero_frames > batched_frames,
        "target 0 must forbid batching: {zero_frames} supply frames vs {batched_frames} batched"
    );
}
