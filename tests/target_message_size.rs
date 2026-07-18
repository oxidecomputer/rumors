//! End-to-end coverage of [`rumors::Peer::target_message_size`]: the knob
//! threads from the public builder into the streaming session, any target
//! (including the degenerate zero) leaves reconciliation convergent, and
//! peers with different targets interoperate because run sizing is not
//! wire-visible.

mod common;

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::{DEFAULT_TARGET_MESSAGE_SIZE, Peer, Rumors};

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
