//! Channel-capacity soundness, structural stress, and scheduled disputes.

use proptest::prelude::*;

use super::fixtures::{
    LeafOrder, divergent_cells_pair, full_depth_comb_pair, one_sided_pair, pyramid_pair,
};
use super::{alternating_mirror, fully_scheduled_streaming_mirror, scheduled_streaming_mirror};
use crate::testing::{Quiescence, run_to_quiescence};
use crate::tree::{
    Root,
    arb::leaf_parent_dispute_pair,
    mirror::streaming::{
        Local, Root as StreamingRoot,
        backend::with_local_schedule,
        materialized::{
            Handshaking,
            channel::{QueueKind, with_kind_capacity, with_observation, with_schedule},
        },
        mirror as drive_streaming,
    },
};

/// Whether the session stalls at a selected capacity for the fan return queue.
fn underbuffered_mirror_stalls(a: Root<()>, b: Root<()>, capacity: usize) -> bool {
    let (a, b): (StreamingRoot<Local, ()>, StreamingRoot<Local, ()>) = (a.into(), b.into());
    let client = Handshaking::start(Local, a);
    let server = Handshaking::start(Local, b);
    with_kind_capacity(QueueKind::AssemblyLevelReturns, capacity, || {
        matches!(
            run_to_quiescence(drive_streaming(client, server)),
            Err(Quiescence::Stalled)
        )
    })
}

/// Check one structural stress case under endpoint and poll-order variations.
fn assert_capacity_case(name: &'static str, pair: (Root<()>, Root<()>)) {
    let (a, b) = pair;
    let expected = alternating_mirror(a.clone(), b.clone());
    let schedules = [
        (Vec::new(), Vec::new()),
        (
            vec![2; 16_384],
            (0..16_384).map(|step| (step % 3) as u8).collect(),
        ),
        (
            (0..16_384).map(|step| (step % 3) as u8).collect(),
            vec![2; 16_384],
        ),
    ];

    for (orientation, left, right) in [("forward", &a, &b), ("reverse", &b, &a)] {
        for (schedule_index, (channel_schedule, backend_schedule)) in schedules.iter().enumerate() {
            let actual = fully_scheduled_streaming_mirror(
                left.clone(),
                right.clone(),
                channel_schedule.clone(),
                backend_schedule.clone(),
            );
            assert_eq!(
                actual, expected,
                "capacity case {name}, {orientation} orientation, schedule {schedule_index}",
            );
        }
    }
}

/// Exact barriers, aggregate overflow, full depth, and multiplying width drain.
#[test]
fn capacity_stress_matrix() {
    // Exactly 256 disputed root children prove that publishing the root
    // resolution first lets one-slot query and return channels stream a fan.
    assert_capacity_case("root full fan", pyramid_pair(&[256], 1, LeafOrder::Outside));

    // A full fan below four simultaneously disputed parents reaches every
    // one-slot recursive query/resolution boundary and the fan-sized
    // inter-level return boundary, with a sibling backlog behind it.
    assert_capacity_case(
        "recursive full fan",
        pyramid_pair(&[4, 256], 1, LeafOrder::Reversed),
    );

    // The exact off-by-one shape and a double-fan variant prove that active
    // draining, rather than an accidentally oversized constant, carries an
    // unbounded sequence of independently resolved children upward.
    assert_capacity_case(
        "fan plus one",
        one_sided_pair(&[(0x00, 254, 1), (0x01, 1, 1)]),
    );
    assert_capacity_case(
        "two aggregate fans",
        one_sided_pair(&[(0x00, 255, 1), (0x01, 255, 1)]),
    );

    // Multiplying widths load several pipeline levels at once. Interleaving
    // the leaf radices prevents request/match/provision order from being an
    // accidental source of progress.
    assert_capacity_case(
        "multiplying pyramid",
        pyramid_pair(&[8, 4, 4, 2], 2, LeafOrder::Interleaved),
    );

    // The comb reaches all 32 trie heights with only linear leaf growth and a
    // disputed sibling branching away from the spine at every internal level.
    assert_capacity_case(
        "full depth comb",
        full_depth_comb_pair(2, LeafOrder::Interleaved),
    );
}

/// Every named, height-carrying queue is exercised at its documented capacity.
#[test]
fn capacity_stress_covers_every_queue_role() {
    let (pair, report) = with_observation(|| {
        let (a, b) = pyramid_pair(&[4, 4, 2], 2, LeafOrder::Interleaved);
        let pair = scheduled_streaming_mirror(a, b, vec![2; 16_384]);
        let (a, b, _) = leaf_parent_dispute_pair();
        scheduled_streaming_mirror(a, b, vec![2; 16_384]);
        pair
    });
    drop(pair);

    for kind in QueueKind::ALL {
        let stats = report.kind(kind);
        assert!(
            stats.channels > 0,
            "queue role {kind:?} was not constructed"
        );
        assert!(stats.sends > 0, "queue role {kind:?} sent no test item");
        assert!(
            stats.receives > 0,
            "queue role {kind:?} received no test item"
        );
        let expected = if kind == QueueKind::AssemblyLevelReturns {
            256
        } else {
            1
        };
        assert_eq!(
            stats.effective_capacity, expected,
            "queue role {kind:?} did not use its documented capacity"
        );
        assert!(
            stats.high_water <= expected,
            "queue role {kind:?} exceeded its effective capacity: {stats:?}"
        );
    }

    let internal_heights = report
        .roles()
        .filter(|(role, _)| role.kind == QueueKind::InternalChildQueries)
        .map(|(role, _)| role.height)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        internal_heights.len() > 1,
        "recursive queue observations lost their typed heights: {internal_heights:?}"
    );
    assert!(
        report
            .roles()
            .any(|(_, stats)| stats.blocked_send_polls > 0),
        "the scheduled run never applied backpressure to a sender"
    );
}

/// The recursive witness proves that the inter-level return queue needs a fan.
#[test]
fn capacity_stress_witness_requires_inter_level_fan() {
    let (a, b) = pyramid_pair(&[32, 256], 1, LeafOrder::Reversed);
    let expected = alternating_mirror(a.clone(), b.clone());
    let (actual, report) =
        with_observation(|| scheduled_streaming_mirror(a.clone(), b.clone(), vec![2; 16_384]));
    assert_eq!(
        actual, expected,
        "the full-fan witness must complete at the documented capacities",
    );
    assert!(
        report.kind(QueueKind::AssemblyLevelReturns).high_water >= 254,
        "the witness did not create its expected near-fan return backlog: {:?}",
        report.kind(QueueKind::AssemblyLevelReturns),
    );
    assert!(
        underbuffered_mirror_stalls(a.clone(), b.clone(), 253),
        "the stress witness no longer stalls just below its required return capacity",
    );
    assert!(
        !underbuffered_mirror_stalls(a, b, 254),
        "the stress witness should complete once its near-fan return backlog fits",
    );
}

/// Whether one shape stalls at a return capacity under explicit poll schedules.
fn shape_stalls(
    pair: &(Root<()>, Root<()>),
    capacity: usize,
    channel_schedule: Vec<u8>,
    backend_schedule: Vec<u8>,
) -> bool {
    let (a, b): (StreamingRoot<Local, ()>, StreamingRoot<Local, ()>) =
        (pair.0.clone().into(), pair.1.clone().into());
    let client = Handshaking::start(Local, a);
    let server = Handshaking::start(Local, b);
    with_kind_capacity(QueueKind::AssemblyLevelReturns, capacity, || {
        with_schedule(channel_schedule, || {
            with_local_schedule(backend_schedule, || {
                matches!(
                    run_to_quiescence(drive_streaming(client, server)),
                    Err(Quiescence::Stalled)
                )
            })
        })
    })
}

/// The poll-order variations each parent-delay probe shape runs under.
fn probe_schedules() -> [(Vec<u8>, Vec<u8>); 5] {
    [
        (Vec::new(), Vec::new()),
        (
            vec![2; 16_384],
            (0..16_384).map(|s| (s % 3) as u8).collect(),
        ),
        (
            (0..16_384).map(|s| (s % 3) as u8).collect(),
            vec![2; 16_384],
        ),
        ((0..16_384).map(|s| (s % 5) as u8).collect(), Vec::new()),
        (vec![1; 16_384], vec![1; 16_384]),
    ]
}

/// Whether a shape stalls under any probe schedule at the given capacity.
fn stalls_under_any_schedule(pair: &(Root<()>, Root<()>), capacity: usize) -> bool {
    probe_schedules()
        .into_iter()
        .any(|(chan, back)| shape_stalls(pair, capacity, chan, back))
}

/// Probe (model finding #7): a lone parent scope stalls the real encoder
/// exactly when its disputed fan exceeds the return capacity plus two.
///
/// The formal model's `Control.pdelay` deadlocks a parent-delaying publisher
/// at disputed fan = capacity + 2. The real encoder IS a parent-delaying
/// publisher (`levels.rs` sends the parent resolution in the scope epilogue,
/// after the final child's dependent queries), yet at fan = capacity + 2 it
/// completes under every probed poll order: the model's committed adversary
/// explores interleavings the sequential reaction loop cannot produce. The
/// encoder's stall boundary is the capacity-tightness law's (fan ≤ cap + 2),
/// pinned here from both sides at cap 1.
#[test]
fn parent_delay_single_parent_boundary() {
    // An internal scope with n disputed children, each carrying leaf-level
    // grandchildren, so dependent queries follow the final child resolution
    // while the enclosing parent resolution waits in the scope epilogue.
    let internal_fan = |n: u8| {
        let cells: Vec<Vec<u8>> = (0..n).map(|radix| vec![0, radix]).collect();
        divergent_cells_pair(&cells, 1, LeafOrder::Outside)
    };

    assert!(
        !stalls_under_any_schedule(&internal_fan(3), 1),
        "fan = cap + 2 must complete: the model's tighter pdelay boundary \
         is not realizable in the sequential encoder"
    );
    assert!(
        stalls_under_any_schedule(&internal_fan(4), 1),
        "fan = cap + 3 must stall: the tightness law's boundary has moved"
    );
    assert!(
        !stalls_under_any_schedule(&internal_fan(4), 2),
        "fan = cap + 2 must complete at the wider capacity too"
    );
}

/// Probe (model finding #7): return backlog does not accumulate across
/// sibling parent scopes at one height.
///
/// The level return queue serves a whole stage, so if completions from
/// several parents' subtrees could occupy it simultaneously, a stage much
/// wider than the queue would stall even with every individual fan inside
/// the tightness boundary — and no fixed capacity would be safe for large
/// trees. The walk's sequential scope order (each parent resolution departs
/// before the next scope's prologue) prevents that: many parents at fan =
/// cap + 2 behave exactly like one.
#[test]
fn parent_delay_no_cross_parent_backlog() {
    let parents_of_three = |parents: u8| {
        let cells: Vec<Vec<u8>> = (0..parents)
            .flat_map(|parent| (0..3u8).map(move |radix| vec![parent, radix]))
            .collect();
        divergent_cells_pair(&cells, 1, LeafOrder::Outside)
    };

    // Control: growing the count of fan-3 parents under ONE root scope grows
    // the ROOT's own fan, so the per-scope law (fan ≤ cap + 2) predicts the
    // stall boundary — confirmed empirically (P=4 stalls at C=1, P=6 at C≤3,
    // P=8 at C≤4, P=12 at C≤6: exactly P > C + 2 throughout).
    assert!(
        stalls_under_any_schedule(&parents_of_three(4), 1),
        "a fan-4 root over fan-3 parents must stall at cap 1 per the \
         per-scope tightness law"
    );
    assert!(
        !stalls_under_any_schedule(&parents_of_three(4), 2),
        "a fan-4 root over fan-3 parents must complete at cap 2 per the \
         per-scope tightness law"
    );

    // The width test proper: every scope's fan stays at 3 (= cap + 2 at
    // cap 1) while stage width grows geometrically (9, 27, 81 scopes). If
    // backlog accumulated across sibling parents, the wide stages would
    // stall; completion at every width shows the sequential scope order
    // confines return backlog to one scope's fan, so a fixed return
    // capacity is safe at any tree width.
    for depth in [2usize, 3, 4] {
        let widths = vec![3usize; depth];
        let pair = pyramid_pair(&widths, 1, LeafOrder::Outside);
        assert!(
            !stalls_under_any_schedule(&pair, 1),
            "a width-{} stage of fan-3 scopes must complete at cap 1: \
             return backlog must not span sibling parents",
            3usize.pow(depth as u32),
        );
    }
}

/// Generate shrinkable structured fan-out without exponential test cases.
fn arb_stress_widths() -> impl Strategy<Value = Vec<usize>> {
    let compact = proptest::collection::vec(1usize..=4, 1..=6).prop_filter(
        "the cartesian pyramid must stay within 128 deepest cells",
        |widths| widths.iter().product::<usize>() <= 128,
    );
    let boundary =
        (0usize..=30, prop_oneof![Just(255usize), Just(256usize)]).prop_map(|(depth, width)| {
            let mut widths = vec![1; depth + 1];
            widths[depth] = width;
            widths
        });
    prop_oneof![4 => compact, 1 => boundary]
}

/// Generate every meaningful relative order of matches, supplies, and queries.
fn arb_leaf_order() -> impl Strategy<Value = LeafOrder> {
    prop_oneof![
        Just(LeafOrder::Outside),
        Just(LeafOrder::Reversed),
        Just(LeafOrder::Interleaved),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        max_shrink_iters: 8_192,
        ..ProptestConfig::default()
    })]

    /// Structured disputes terminate under independently shrinkable channel
    /// and Local-backend poll schedules.
    #[test]
    fn scheduled_structured_disputes_match_oracle(
        widths in arb_stress_widths(),
        shared in 1usize..=3,
        order in arb_leaf_order(),
        channel_schedule in proptest::collection::vec(0u8..=2, 0..=2_048),
        backend_schedule in proptest::collection::vec(0u8..=2, 0..=2_048),
        reverse in any::<bool>(),
    ) {
        let (a, b) = pyramid_pair(&widths, shared, order);
        let expected = alternating_mirror(a.clone(), b.clone());
        let (left, right) = if reverse { (b, a) } else { (a, b) };
        let actual = fully_scheduled_streaming_mirror(
            left,
            right,
            channel_schedule,
            backend_schedule,
        );
        prop_assert_eq!(actual, expected);
    }
}
