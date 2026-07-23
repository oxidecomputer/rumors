//! End-to-end coverage of [`rumors::Peer::target_message_size`]: the knob
//! threads from the public builder into the greeting, the session runs at
//! the exchanged minimum of the two sides' settings, any minimum (including
//! the degenerate zero) leaves reconciliation convergent, and the minimum
//! binds both encoders regardless of which side advertised it.

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
/// session runs: each greeting advertises its side's setting, and every
/// session between them runs at the exchanged minimum.
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

/// A zero target degrades run batching to one leaf per message; the
/// session still reconciles a divergent pair completely.
#[test]
fn zero_target_still_converges() {
    assert_converges(diverged_pair(0, 0));
}

/// Peers with different targets interoperate: the session runs at the
/// exchanged minimum of the two greeting-carried settings — here zero, so
/// both encoders unbatch.
#[test]
fn mixed_targets_interoperate() {
    assert_converges(diverged_pair(0, DEFAULT_TARGET_MESSAGE_SIZE));
}

/// Like [`diverged_pair`], but with deterministically seeded peer identities,
/// so the same divergence replays byte-identically across sessions and their
/// wire captures are comparable.
fn seeded_diverged_pair(
    left_target: usize,
    right_target: usize,
    (left_messages, right_messages): (usize, usize),
) -> (Rumors<u64>, Rumors<u64>) {
    block_on(async {
        let left = Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
            .target_message_size(left_target)
            .into_rumors();
        let right = bootstrap_fork_async(&left).await;
        let right = right
            .try_into_peer()
            .await
            .expect("the fresh fork has a single handle")
            .target_message_size(right_target)
            .into_rumors();

        let mut rng = SmallRng::seed_from_u64(0x5eed_0f1e_a55e_d000);
        for _ in 0..left_messages {
            left.send(rng.next_u64());
        }
        for _ in 0..right_messages {
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
        let (left, right) = seeded_diverged_pair(0, 0, (DIVERGENT_PER_SIDE, DIVERGENT_PER_SIDE));
        capture_gossip(left, right)
    };
    let batched = {
        let (left, right) = seeded_diverged_pair(
            DEFAULT_TARGET_MESSAGE_SIZE,
            DEFAULT_TARGET_MESSAGE_SIZE,
            (DIVERGENT_PER_SIDE, DIVERGENT_PER_SIDE),
        );
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

/// A small nonzero target: a few supply records fit one run, so a session
/// bound by it emits strictly more frames than the default target and
/// strictly fewer than an unbatched (zero-target) one.
const SMALL_TARGET: usize = 128;

/// Messages the supplying side originates for the binding-minimum capture:
/// dense enough that the root's 256 child subtrees average eight novel
/// leaves each, so the runs supplying them outgrow [`SMALL_TARGET`] and the
/// small bound genuinely splits them.
const BINDING_MESSAGES: usize = 2048;

/// Messages the other side originates: one, so the divergence is two-sided
/// and disputes resolve at the root fan (whole one-byte-prefix subtrees are
/// supplied as single runs), rather than against an empty replica, whose
/// dispute shape supplies narrower subtrees.
const BINDING_COUNTER_MESSAGES: usize = 1;

/// The exchanged minimum binds both encoders when nonzero, which the
/// zero-minimum cell cannot show.
///
/// The same seeded one-sided divergence is reconciled under five target
/// assignments. If the session runs at the exchanged minimum, a mixed
/// (small, default) pair must emit exactly the frame count of a
/// (small, small) pair — in particular, a default-target supplier facing a
/// small-target receiver is bound by the *remote* setting — and the same
/// count with the assignment reversed. The small-bound count must sit
/// strictly between the default-bound count (small genuinely splits runs)
/// and the zero-target count (a nonzero minimum still batches). If run
/// sizing were each encoder's local choice, the mixed captures would
/// differ from the uniform small capture.
#[test]
fn nonzero_minimum_binds_both_encoders() {
    let frames = |left, right| {
        let (l, r) =
            seeded_diverged_pair(left, right, (BINDING_MESSAGES, BINDING_COUNTER_MESSAGES));
        supply_frames(&capture_gossip(l, r))
    };
    let small_default = frames(SMALL_TARGET, DEFAULT_TARGET_MESSAGE_SIZE);
    let default_small = frames(DEFAULT_TARGET_MESSAGE_SIZE, SMALL_TARGET);
    let small_small = frames(SMALL_TARGET, SMALL_TARGET);
    let default_default = frames(DEFAULT_TARGET_MESSAGE_SIZE, DEFAULT_TARGET_MESSAGE_SIZE);
    let zero_zero = frames(0, 0);

    assert_eq!(
        small_default, small_small,
        "the default-target side must encode at the exchanged minimum"
    );
    assert_eq!(
        default_small, small_small,
        "the minimum must bind regardless of which side advertised it"
    );
    assert!(
        small_small > default_default,
        "the small target must genuinely split runs: \
         {small_small} frames vs {default_default} at the default"
    );
    assert!(
        small_small < zero_zero,
        "a nonzero minimum must still batch: \
         {small_small} frames vs {zero_zero} unbatched"
    );
}
