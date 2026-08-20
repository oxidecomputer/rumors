//! End-to-end coverage of [`rumors::Peer::target_message_size`] and its
//! bootstrap-time twin [`rumors::Bootstrap::target_message_size`].
//!
//! The knob
//! threads from the public builders into the greeting, the session runs at
//! the exchanged minimum of the two sides' settings, any minimum (including
//! the degenerate zero) leaves reconciliation convergent, and the minimum
//! binds both encoders regardless of which side advertised it.

mod common;

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::{Bootstrap, DEFAULT_TARGET_MESSAGE_SIZE, Peer, Rumors};

use crate::common::gossip_snapshot::{capture_gossip, capture_session, observed};
use crate::common::wire::{block_on, bootstrap_fork_async, wire_gossip_async};

/// Messages each side originates after the fork: enough for multi-leaf
/// supplied subtrees, so run batching is genuinely exercised on the wire.
const DIVERGENT_PER_SIDE: usize = 96;

/// A divergent pair whose sides select different run targets before any
/// session runs: each greeting advertises its side's setting, and every
/// session between them runs at the exchanged minimum.
fn diverged_pair(left_target: usize, right_target: usize) -> (Rumors<u64>, Rumors<u64>) {
    block_on(async {
        let left = Peer::seed()
            .sync_window_floor()
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
        left.iter().collect::<Vec<_>>(),
        right.iter().collect::<Vec<_>>(),
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
            .sync_window_floor()
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
///
/// Oracle assumption: this counts frames the capture labels `Supply` and
/// nothing else, so it observes run sizing only while supplied leaves ride
/// frames with that label. A protocol change that relabels leaf-bearing
/// frames would weaken every count-based assertion here silently; the
/// snapshot suite (`tests/gossip_snapshot.rs`) pins the labels themselves.
fn supply_frames(capture: &str) -> usize {
    capture
        .lines()
        .filter(|line| is_supply_signal(line))
        .count()
}

/// Whether one rendered line is a supply frame's signal line: a bare
/// dense code followed by a `/ Supply… /` comment. The bare-digit code
/// distinguishes signal lines from every other annotated line.
fn is_supply_signal(line: &str) -> bool {
    let Some((code, rest)) = line.trim_start().split_once(" / ") else {
        return false;
    };
    !code.is_empty() && code.bytes().all(|b| b.is_ascii_digit()) && rest.starts_with("Supply")
}

/// Count the supply frames in each direction of a rendered wire capture:
/// `(a_to_b, b_to_a)`, attributed by the capture's `direction` headers.
/// Same oracle assumption as [`supply_frames`].
fn directional_supply_frames(capture: &str) -> (usize, usize) {
    let (mut a_to_b, mut b_to_a) = (0, 0);
    let mut toward_b = true;
    for line in capture.lines() {
        if line.starts_with("direction A -> B") {
            toward_b = true;
        } else if line.starts_with("direction B -> A") {
            toward_b = false;
        } else if is_supply_signal(line) {
            if toward_b {
                a_to_b += 1;
            } else {
                b_to_a += 1;
            }
        }
    }
    (a_to_b, b_to_a)
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

/// A small nonzero target that fits one supply record of this corpus but
/// not two: every multi-record run splits under it, in both supply
/// directions, so both directions' margin self-checks below have teeth.
///
/// (A nonzero target still batching runs that fit is
/// [`zero_target_emits_more_supply_frames_than_default`]'s claim, over a
/// corpus whose runs fit the default; this test's corpus is chosen for
/// per-direction discrimination instead.)
const SMALL_TARGET: usize = 32;

/// Messages each side originates for the binding-minimum capture: two per
/// root-fan child on average.
///
/// At this shape both supply directions ship
/// their exclusive root-fan subtrees as whole multi-record runs — the
/// responder in its opening reply, the initiator in its opening supplies —
/// so the margin self-checks hold per direction and the tuple equalities
/// discriminate in both directions.
const BINDING_MESSAGES_PER_SIDE: usize = 512;

/// The exchanged minimum binds both encoders when nonzero, which the
/// zero-minimum cell cannot show.
///
/// The same seeded two-sided divergence is reconciled under four target
/// assignments, and supply frames are counted per direction. Under the
/// exchanged-minimum semantics every cell containing a small advertisement
/// runs both directions at the small target, so both mixed cells must equal
/// the uniform-small cell as `(a_to_b, b_to_a)` tuples exactly. Under the
/// rejected reading — each encoder sizing runs by its own local setting —
/// each mixed cell would differ from the uniform-small cell in exactly the
/// component its default-advertising side supplies: (small, default) in
/// `b_to_a`, (default, small) in `a_to_b`, each landing at the
/// uniform-default cell's count for that component instead. The margin
/// self-checks prove those components genuinely differ between the
/// uniform-small and uniform-default cells, so neither equality can hold
/// vacuously.
#[test]
fn nonzero_minimum_binds_both_encoders() {
    let frames = |left, right| {
        let (l, r) = seeded_diverged_pair(
            left,
            right,
            (BINDING_MESSAGES_PER_SIDE, BINDING_MESSAGES_PER_SIDE),
        );
        directional_supply_frames(&capture_gossip(l, r))
    };
    let small_default = frames(SMALL_TARGET, DEFAULT_TARGET_MESSAGE_SIZE);
    let default_small = frames(DEFAULT_TARGET_MESSAGE_SIZE, SMALL_TARGET);
    let small_small = frames(SMALL_TARGET, SMALL_TARGET);
    let default_default = frames(DEFAULT_TARGET_MESSAGE_SIZE, DEFAULT_TARGET_MESSAGE_SIZE);

    // Margin self-checks: the small target splits runs in each direction
    // independently, so the tuple equalities below cannot hold vacuously.
    assert!(
        small_small.0 > default_default.0,
        "A -> B runs must outgrow the small target: \
         {} frames small vs {} default",
        small_small.0,
        default_default.0,
    );
    assert!(
        small_small.1 > default_default.1,
        "B -> A runs must outgrow the small target: \
         {} frames small vs {} default",
        small_small.1,
        default_default.1,
    );

    assert_eq!(
        small_default, small_small,
        "side B advertises the default but must supply at the remote small \
         minimum: under own-setting sizing this cell's b_to_a count would \
         match the uniform-default cell's"
    );
    assert_eq!(
        default_small, small_small,
        "side A advertises the default but must supply at the remote small \
         minimum: under own-setting sizing this cell's a_to_b count would \
         match the uniform-default cell's"
    );
}

/// A deterministically seeded, populated provider: the same corpus for
/// every bootstrap capture, so their supply-frame counts are comparable.
fn seeded_provider() -> Rumors<u64> {
    let provider = Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
        .sync_window_floor()
        .into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x5eed_0f1e_a55e_d000);
    for _ in 0..DIVERGENT_PER_SIDE {
        provider.send(rng.next_u64());
    }
    provider
}

/// Capture one bootstrap session's wire traffic: `provider` serves via
/// `gossip` while a newcomer joins under `config`.
fn capture_bootstrap(provider: Rumors<u64>, config: Bootstrap<u64>) -> String {
    capture_session(
        move |mut link, hook| async move {
            let provider = observed(provider, hook).await;
            provider
                .gossip(&mut link)
                .await
                .expect("the provider serves the bootstrap");
        },
        move |mut link, hook| async move {
            config
                .observe(hook)
                .join(&mut link)
                .await
                .expect("the bootstrap session completes")
                .expect("the provider is established, not itself bootstrapping");
        },
    )
}

/// A budget far from [`rumors::DEFAULT_SYNC_MEMORY_BUDGET`]'s 512 MiB, so
/// the wire-equality pin below cannot pass by the two configurations
/// coinciding.
const OFF_DEFAULT_BUDGET: usize = 1024 * 1024;

/// A configured sync memory budget leaves the bootstrap wire byte-identical
/// to the default's: the budget is deliberately not wire-visible, so peers
/// with different budgets interoperate.
///
/// The same seeded provider serves the same corpus to a default-budget
/// newcomer and to one advertising [`OFF_DEFAULT_BUDGET`]; the full wire
/// capture (both directions) must be equal byte for byte. This pins the
/// structural guarantee that the greeting does not carry the budget —
/// each session derives its window locally from the exchanged sizes — so
/// a regression that leaks the budget onto the wire fails here rather
/// than surfacing as an interoperability break between fleets rolled out
/// with different budgets. The negative control proves the comparator has
/// teeth: under the identical harness, a knob that *is* wire-visible
/// (the greeting carries the message-size target) must change the
/// capture, so a comparator bug that always reports equality fails the
/// control.
#[test]
fn sync_memory_budget_is_not_wire_visible() {
    let default_budget = capture_bootstrap(seeded_provider(), Peer::<u64>::bootstrap());
    let custom_budget = capture_bootstrap(
        seeded_provider(),
        Peer::<u64>::bootstrap().sync_memory_budget(OFF_DEFAULT_BUDGET),
    );
    assert_eq!(
        default_budget, custom_budget,
        "a configured sync memory budget must not change one wire byte of the session",
    );

    // Negative control: the same harness must be able to see a difference.
    let custom_target = capture_bootstrap(
        seeded_provider(),
        Peer::<u64>::bootstrap().target_message_size(0),
    );
    assert_ne!(
        default_budget, custom_target,
        "the wire-visible target knob must change the capture: an equality here \
         means the comparator cannot distinguish sessions at all",
    );
}

/// A newcomer's [`Bootstrap::target_message_size`] genuinely reaches the
/// bootstrap greeting and binds the *provider's* encoder, which no
/// post-bootstrap knob could show.
///
/// The same seeded provider serves the same corpus twice: to a newcomer
/// advertising a zero target (batching forbidden: one leaf per supply
/// frame) and to an unconfigured newcomer (the default target batches each
/// supplied subtree's leaves into runs). The zero-target join must draw
/// strictly more supply frames out of the provider. The default-target
/// capture is the negative control: if the builder's advertisement never
/// reached the greeting, both sessions would run at the provider's own
/// setting and the two counts would be equal, failing the strict
/// inequality.
#[test]
fn bootstrap_advertisement_binds_the_provider() {
    let unbatched = capture_bootstrap(
        seeded_provider(),
        Peer::<u64>::bootstrap().target_message_size(0),
    );
    let batched = capture_bootstrap(seeded_provider(), Peer::<u64>::bootstrap());
    let (unbatched_frames, batched_frames) = (supply_frames(&unbatched), supply_frames(&batched));
    assert!(
        batched_frames > 0,
        "a populated bootstrap must supply at least one run"
    );
    assert!(
        unbatched_frames > batched_frames,
        "a zero bootstrap advertisement must forbid the provider's batching: \
         {unbatched_frames} supply frames vs {batched_frames} batched"
    );
}
