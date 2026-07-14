use std::time::Duration;

use proptest::prelude::*;

use crate::Version;
use crate::message::Message;
use crate::tree::Root;
use crate::tree::arb::{
    arb_divergent_pair, arb_tree_root, leaf_parent_dispute_pair, leaf_parent_redaction_pair,
    nth_party,
};
use crate::tree::mirror::alternating;
use crate::tree::mirror::streaming::materialized::channel::{with_capacity_limit, with_schedule};
use crate::tree::mirror::streaming::{
    Handshaking, Local, Root as StreamingRoot, mirror as drive_streaming,
};
use crate::tree::traverse::{Action, act};
use crate::tree::typed::{Node as TreeNode, Path, height};

/// Reconcile `a` and `b` through the streaming local backend, returning both
/// sides' reconciled roots in argument order, with no convergence assertion.
fn streaming_mirror_sides(a: Root<()>, b: Root<()>) -> (Root<()>, Root<()>) {
    streaming_mirror_sides_with_schedule(a, b, Vec::new(), Duration::from_secs(5))
}

/// Reconcile under an explicit channel-poll schedule and cancellable timeout.
fn streaming_mirror_sides_with_schedule(
    a: Root<()>,
    b: Root<()>,
    schedule: Vec<u8>,
    timeout: Duration,
) -> (Root<()>, Root<()>) {
    let (a, b): (StreamingRoot<Local, ()>, StreamingRoot<Local, ()>) = (a.into(), b.into());
    let client = Handshaking::start(Local, a.clone());
    let server = Handshaking::start(Local, b.clone());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("the test runtime should build");
    let result = with_schedule(schedule, || {
        runtime.block_on(async move {
            tokio::time::timeout(timeout, drive_streaming(client, server)).await
        })
    });
    let (ours, theirs) = result
        .expect("streaming mirror made no progress before its timeout")
        // `Local` is infallible, so the session's only inhabited errors are
        // violations — which two honest local endpoints must never speak.
        .expect("local mirror speaks no violations")
        // Equal handshake versions: already converged, both sides unchanged.
        .unwrap_or((a, b));
    (ours.into(), theirs.into())
}

/// Reconcile `a` and `b` through the streaming local backend, asserting the
/// two sides converge to the same root, and return it.
fn streaming_mirror(a: Root<()>, b: Root<()>) -> Root<()> {
    let (ours, theirs) = streaming_mirror_sides(a, b);
    assert_eq!(ours, theirs, "streaming endpoints should converge");
    ours
}

/// Reconcile under an explicit channel-poll schedule, asserting convergence.
fn scheduled_streaming_mirror(
    a: Root<()>,
    b: Root<()>,
    schedule: Vec<u8>,
    timeout: Duration,
) -> Root<()> {
    let (ours, theirs) = streaming_mirror_sides_with_schedule(a, b, schedule, timeout);
    assert_eq!(
        ours, theirs,
        "scheduled streaming endpoints should converge"
    );
    ours
}

/// Whether the same session parks when the fan-sized return queue is forced to one.
fn underbuffered_mirror_times_out(a: Root<()>, b: Root<()>) -> bool {
    let (a, b): (StreamingRoot<Local, ()>, StreamingRoot<Local, ()>) = (a.into(), b.into());
    let client = Handshaking::start(Local, a);
    let server = Handshaking::start(Local, b);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("the test runtime should build");
    with_capacity_limit(1, || {
        runtime.block_on(async move {
            tokio::time::timeout(Duration::from_millis(500), drive_streaming(client, server))
                .await
                .is_err()
        })
    })
}

/// Reconcile `a` and `b` through the alternating implementation — the
/// behavioral oracle the streaming protocol must reproduce exactly —
/// returning both sides' roots in argument order, with no convergence
/// assertion.
fn alternating_mirror_sides(a: Root<()>, b: Root<()>) -> (Root<()>, Root<()>) {
    pollster::block_on(async {
        let local_a = alternating::local::Exchange::start(a);
        let local_b = alternating::local::Exchange::start(b);
        match alternating::mirror(local_a, local_b).await {
            Err(e) => match e {},
            Ok(pair) => pair,
        }
    })
}

/// Reconcile `a` and `b` through the alternating oracle, asserting the two
/// sides converge to the same root, and return it.
fn alternating_mirror(a: Root<()>, b: Root<()>) -> Root<()> {
    let (ours, theirs) = alternating_mirror_sides(a, b);
    assert_eq!(ours, theirs, "oracle endpoints should converge");
    ours
}

/// Build a divergent pair whose every difference is one-sided, shaped by
/// `spec`.
///
/// For each `(radix, shared, extra)` root child, both trees hold `shared`
/// identical leaves under it and `b` additionally holds `extra` concurrent
/// ones.
///
/// Leaves are placed at hand-picked paths (first byte the root radix, second
/// byte a counter), not content-addressed ones: the reconciliation machinery
/// keys purely by prefix, and controlling the first two bytes is what lets a
/// test pin the exact fan-out each walk routes. Because no key is present on
/// both sides with different content, every root child disputes but nothing
/// disputes below it: the session's descent is empty, and the whole diff
/// resolves in the first descending stage.
fn one_sided_pair(spec: &[(u8, u8, u8)]) -> (Root<()>, Root<()>) {
    let path = |b0: u8, b1: u8| {
        let mut bytes = [0u8; 32];
        bytes[0] = b0;
        bytes[1] = b1;
        Path::from(bytes)
    };

    // The shared base: one version chain on party 0, identical in both trees
    // (b is built on top of a's node, so the shared subtrees are literally
    // the same nodes and their hashes match by construction).
    let shared_party = nth_party(0);
    let mut version = Version::new();
    let mut shared = Vec::new();
    for &(radix, n_shared, _) in spec {
        for i in 0..n_shared {
            version.tick(&shared_party);
            shared.push((
                path(radix, i),
                version.clone(),
                Action::Insert(Message::new(())),
            ));
        }
    }
    let a_node = act(None, shared, |_| ());

    // b's extras: a separate chain on a disjoint party, so they are causally
    // concurrent with a's version and survive deletion-pruning when provided.
    // Extras count down from 0xff so they never collide with a shared radix.
    let b_party = nth_party(1);
    let mut b_version = Version::new();
    let mut extras = Vec::new();
    for &(radix, _, n_extra) in spec {
        for i in 0..n_extra {
            b_version.tick(&b_party);
            extras.push((
                path(radix, 0xff - i),
                b_version.clone(),
                Action::Insert(Message::new(())),
            ));
        }
    }
    let b_node = act(a_node.clone(), extras, |_| ());

    let root = |node: Option<TreeNode<(), height::Root>>| Root {
        ceiling: node
            .as_ref()
            .map(TreeNode::ceiling)
            .cloned()
            .unwrap_or_default(),
        root: node,
    };
    (root(a_node), root(b_node))
}

/// The radix ordering of shared leaves and each side's extra leaf.
#[derive(Clone, Copy, Debug)]
enum LeafOrder {
    /// `a`'s extra, shared run, then `b`'s extra.
    Outside,
    /// `b`'s extra, shared run, then `a`'s extra.
    Reversed,
    /// Extras interspersed with the shared run.
    Interleaved,
}

impl LeafOrder {
    fn slots(self, shared: usize) -> (Vec<u8>, u8, u8) {
        assert!((1..=100).contains(&shared));
        match self {
            Self::Outside => ((1..=shared as u8).collect(), 0x00, 0xff),
            Self::Reversed => ((1..=shared as u8).collect(), 0xff, 0x00),
            Self::Interleaved => (
                (1..=shared as u8).map(|slot| slot * 2).collect(),
                0x03,
                0x01,
            ),
        }
    }
}

/// Build a bidirectionally divergent pair over explicitly chosen prefix cells.
fn divergent_cells_pair(
    cells: &[Vec<u8>],
    shared: usize,
    order: LeafOrder,
) -> (Root<()>, Root<()>) {
    assert!(cells.iter().all(|cell| cell.len() < 32));
    let (shared_slots, a_slot, b_slot) = order.slots(shared);
    let path = |cell: &[u8], slot: u8| {
        let mut bytes = [0u8; 32];
        bytes[..cell.len()].copy_from_slice(cell);
        bytes[cell.len()] = slot;
        Path::from(bytes)
    };

    // The shared base: one version chain on party 0, identical in both trees
    // (both sides are built on top of the same base node, so the shared
    // subtrees are literally the same nodes and their hashes match by
    // construction).
    let shared_party = nth_party(0);
    let mut version = Version::new();
    let mut base = Vec::new();
    for cell in cells {
        for &slot in &shared_slots {
            version.tick(&shared_party);
            base.push((
                path(cell, slot),
                version.clone(),
                Action::Insert(Message::new(())),
            ));
        }
    }
    let base_node = act(None, base, |_| ());

    // Each side's extras ride their own party's chain, concurrent with the
    // shared chain and with each other, so both survive deletion-pruning
    // when provided across.
    let extras = |party_index: usize, slot: u8| {
        let party = nth_party(party_index);
        let mut version = Version::new();
        let mut actions = Vec::new();
        for cell in cells {
            version.tick(&party);
            actions.push((
                path(cell, slot),
                version.clone(),
                Action::Insert(Message::new(())),
            ));
        }
        actions
    };
    let a_node = act(base_node.clone(), extras(2, a_slot), |_| ());
    let b_node = act(base_node, extras(1, b_slot), |_| ());

    let root = |node: Option<TreeNode<(), height::Root>>| Root {
        ceiling: node
            .as_ref()
            .map(TreeNode::ceiling)
            .cloned()
            .unwrap_or_default(),
        root: node,
    };
    (root(a_node), root(b_node))
}

/// Build a cartesian pyramid whose disputes descend every controlled level.
fn pyramid_pair(widths: &[usize], shared: usize, order: LeafOrder) -> (Root<()>, Root<()>) {
    assert!(widths.iter().all(|&width| (1..=256).contains(&width)));
    let mut cells: Vec<Vec<u8>> = vec![Vec::new()];
    for &width in widths {
        cells = cells
            .into_iter()
            .flat_map(|cell| {
                (0..width as u16).map(move |radix| {
                    let mut cell = cell.clone();
                    cell.push(radix as u8);
                    cell
                })
            })
            .collect();
    }
    divergent_cells_pair(&cells, shared, order)
}

/// Build a linear-size comb with a dispute branching from every trie level.
fn full_depth_comb_pair(shared: usize, order: LeafOrder) -> (Root<()>, Root<()>) {
    let mut cells = vec![vec![0; 31]];
    for depth in 0..31 {
        let mut cell = vec![0; 31];
        cell[depth] = 1;
        cells.push(cell);
    }
    divergent_cells_pair(&cells, shared, order)
}

/// A dispute that survives to leaf-parent height — both sides hold the same
/// `S<Z>` prefix with different leaf sets — converges to the union.
///
/// The responder's closing `uncertain` lists its leaves, and the leaf-height
/// `Closing`/`Complete` words carry the difference in both directions.
#[test]
fn converges_on_leaf_parent_dispute() {
    let (a, b, expected) = leaf_parent_dispute_pair();
    assert_eq!(
        streaming_mirror(a, b),
        expected,
        "both sides should hold the union",
    );
}

/// A leaf redacted on one side under a disputed leaf-parent must disappear
/// from the other side too: the closing request for it prunes against the
/// redactor's version and drops on both sides instead of shipping.
#[test]
fn honors_redaction_under_leaf_parent_dispute() {
    let (a, b, expected) = leaf_parent_redaction_pair();
    assert_eq!(
        streaming_mirror(a, b),
        expected,
        "the redacted leaf should survive nowhere",
    );
}

/// Check one structural stress case under endpoint and poll-order variations.
fn assert_capacity_case(name: &'static str, pair: (Root<()>, Root<()>)) {
    let (a, b) = pair;
    let expected = alternating_mirror(a.clone(), b.clone());
    let schedules = [
        Vec::new(),
        vec![2; 16_384],
        (0..16_384).map(|step| (step % 3) as u8).collect(),
    ];

    for (orientation, left, right) in [("forward", &a, &b), ("reverse", &b, &a)] {
        for (schedule_index, schedule) in schedules.iter().enumerate() {
            let actual = scheduled_streaming_mirror(
                left.clone(),
                right.clone(),
                schedule.clone(),
                Duration::from_secs(10),
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

/// The recursive witness proves that the inter-level return queue needs a fan.
#[test]
fn capacity_stress_witness_requires_inter_level_fan() {
    let (a, b) = pyramid_pair(&[32, 256], 1, LeafOrder::Reversed);
    let expected = alternating_mirror(a.clone(), b.clone());
    assert_eq!(
        scheduled_streaming_mirror(
            a.clone(),
            b.clone(),
            vec![2; 16_384],
            Duration::from_secs(10),
        ),
        expected,
        "the full-fan witness must complete at the documented capacities",
    );
    assert!(
        underbuffered_mirror_times_out(a, b),
        "the stress witness no longer reaches the inter-level fan-capacity deadlock",
    );
}

/// Generate shrinkable structured fan-out without exponential test cases.
fn arb_stress_widths() -> impl Strategy<Value = Vec<usize>> {
    proptest::collection::vec(1usize..=4, 1..=6).prop_filter(
        "the cartesian pyramid must stay within 128 deepest cells",
        |widths| widths.iter().product::<usize>() <= 128,
    )
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
        cases: 48,
        max_shrink_iters: 4_096,
        ..ProptestConfig::default()
    })]

    /// Structured disputes terminate under shrinkable channel-poll schedules.
    #[test]
    fn scheduled_structured_disputes_match_oracle(
        widths in arb_stress_widths(),
        shared in 1usize..=3,
        order in arb_leaf_order(),
        schedule in proptest::collection::vec(0u8..=2, 0..=512),
        reverse in any::<bool>(),
    ) {
        let (a, b) = pyramid_pair(&widths, shared, order);
        let expected = alternating_mirror(a.clone(), b.clone());
        let (left, right) = if reverse { (b, a) } else { (a, b) };
        let actual = scheduled_streaming_mirror(
            left,
            right,
            schedule,
            Duration::from_secs(2),
        );
        prop_assert_eq!(actual, expected);
    }
}

proptest! {
    /// On divergent trees sharing causal history — matched subtrees, one-sided
    /// inserts, and redactions the other side must honor — the streaming
    /// mirror reconciles both sides to exactly the alternating oracle's root.
    #[test]
    fn matches_oracle_on_divergent_pair((a, b) in arb_divergent_pair()) {
        let expected = alternating_mirror(a.clone(), b.clone());
        prop_assert_eq!(streaming_mirror(a, b), expected);
    }

    /// On causally independent trees — including the bootstrap shape, where
    /// one side is empty and receives everything — the streaming mirror
    /// matches the alternating oracle.
    #[test]
    fn matches_oracle_on_independent_trees(
        a in arb_tree_root(0, 0..=8),
        b in arb_tree_root(1, 0..=8),
    ) {
        let expected = alternating_mirror(a.clone(), b.clone());
        prop_assert_eq!(streaming_mirror(a, b), expected);
    }

    /// Mirroring a tree with itself is a no-op: the handshake versions are
    /// equal, the session short-circuits before reconciliation, and both
    /// sides come back unchanged.
    #[test]
    fn idempotent(a in arb_tree_root(0, 0..=8)) {
        prop_assert_eq!(streaming_mirror(a.clone(), a.clone()), a);
    }
}
