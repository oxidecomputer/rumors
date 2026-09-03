//! Session-stats pins at the walk tier: the counters against an
//! independent in-memory oracle.
//!
//! The walk's counters ([`Recorder`]) are pinned here against corpora
//! whose dispute structure is computable by construction: for the
//! `divergent_cells_pair` family, every difference is one-sided under a
//! set of controlled prefix cells, so the session's disputed scopes are
//! exactly the prefix closure of the cells (a scope is disputed iff both
//! sides occupy it and its subtrees differ; differences propagate up the
//! Merkle hashes, and nothing below a cell is jointly occupied where the
//! sides diverge). Per side, the counts split by depth parity: the
//! responder answers the root and every even depth, the initiator every
//! odd depth (the descent alternates askers height by height). The same
//! counters ride the wire unchanged, so what is pinned in memory here is
//! trusted on the wire (the crate's join/mirror equivalence discipline);
//! the public suite (`tests/session_stats.rs`) re-checks the surface.

use std::collections::BTreeSet;

use proptest::prelude::*;

use super::fixtures::{LeafOrder, divergent_cells_pair, grown, path_at, rooted};
use super::{LocalSession, join_oracle};
use crate::tree::Root;
use crate::tree::arb::{
    arb_forgotten_siblings, forgotten_sibling_pair, leaf_parent_redaction_pair,
};
use crate::tree::mirror::streaming::message::initiates;
use crate::tree::mirror::streaming::stats::SessionStats;
use crate::tree::mirror::streaming::{Local, Root as StreamingRoot};

/// Reconcile `a` and `b` through the streaming local backend with a
/// recorder on each side, returning both reconciled roots and both
/// sides' stats in argument order.
///
/// No wire is involved, so the byte counters stay zero here; this
/// harness pins the walk's counters (disputes, gains, sheds) and the
/// window grant.
fn mirror_with_stats(a: Root, b: Root) -> (Root, Root, SessionStats, SessionStats) {
    let outcome = LocalSession::new(a, b).stats().run();
    let (a_stats, b_stats) = *outcome.stats();
    let (ours, theirs) = outcome.sides();
    (ours, theirs, a_stats, b_stats)
}

/// The live-leaf count of a tree root, for conservation checks.
fn live(root: &Root) -> u64 {
    let root: StreamingRoot<Local> = root.clone().into();
    root.len()
}

/// Whether `a` wins the initiator election against `b`, mirroring the
/// session's role election (the smaller exchanged set initiates,
/// canonical version bytes break ties).
fn a_initiates(a: &Root, b: &Root) -> bool {
    let (a, b): (StreamingRoot<Local>, StreamingRoot<Local>) = (a.clone().into(), b.clone().into());
    initiates(a.len(), &a.ceiling, b.len(), &b.ceiling)
}

/// The dispute oracle for `divergent_cells_pair` corpora: the number of
/// disputed scopes at each depth is the prefix closure of the cells,
/// grouped by prefix length.
///
/// Every strict-or-equal prefix of a cell is jointly occupied (both
/// sides hold populations under the cell) with differing subtree hashes
/// (the one-sided extras differ, and hash differences propagate to every
/// ancestor), and nothing else is: below a cell, the extras sit in
/// distinct radix columns, so no divergent slot is jointly occupied.
fn expected_disputes_by_depth(cells: &[Vec<u8>]) -> Vec<usize> {
    let mut prefixes: BTreeSet<Vec<u8>> = BTreeSet::new();
    for cell in cells {
        for len in 0..=cell.len() {
            prefixes.insert(cell[..len].to_vec());
        }
    }
    let deepest = cells.iter().map(Vec::len).max().unwrap_or(0);
    let mut by_depth = vec![0usize; deepest + 1];
    for prefix in prefixes {
        by_depth[prefix.len()] += 1;
    }
    by_depth
}

/// Split a per-depth dispute census into the (initiator, responder)
/// per-side expectation: the responder answers the root and every even
/// depth, the initiator every odd depth.
fn split_by_parity(by_depth: &[usize]) -> (u64, u64) {
    let responder: usize = by_depth.iter().step_by(2).sum();
    let initiator: usize = by_depth.iter().skip(1).step_by(2).sum();
    (initiator as u64, responder as u64)
}

/// Generate an antichain of controlled prefix cells for
/// `divergent_cells_pair`.
///
/// No cell is a prefix of another, and every cell byte sits outside the
/// fixture's slot alphabet, so the closure oracle's occupancy reasoning
/// holds exactly.
fn arb_cells() -> impl Strategy<Value = Vec<Vec<u8>>> {
    proptest::collection::btree_set(proptest::collection::vec(0x66u8..0x7f, 0..=3), 0..=5).prop_map(
        |cells| {
            let all: Vec<Vec<u8>> = cells.into_iter().collect();
            all.iter()
                .filter(|cell| {
                    !all.iter()
                        .any(|other| other.len() < cell.len() && cell.starts_with(other))
                })
                .cloned()
                .collect()
        },
    )
}

/// A converged pair reports zero in every field: equal greeting versions
/// end the session before any descent, so no window is derived, nothing
/// is disputed, and nothing moves.
#[test]
fn equal_trees_report_zero_stats() {
    let node = grown(None, 0, 1, &(), &[path_at(&[0x11]), path_at(&[0x22])]);
    let (a, b) = (rooted(node.clone()), rooted(node));
    let (ours, theirs, a_stats, b_stats) = mirror_with_stats(a, b);
    assert_eq!(ours, theirs, "equal endpoints stay equal");
    assert_eq!(a_stats, SessionStats::default());
    assert_eq!(b_stats, SessionStats::default());
}

/// The deterministic dispute pin over a two-by-two pyramid corpus.
///
/// The session disputes exactly the cells' prefix closure (seven
/// scopes), split five to the responder (depths zero and two) and two
/// to the initiator (depth one), with each side gaining exactly the
/// other's four extras, shedding nothing, and running at the floor
/// window's one-scope width.
#[test]
fn pyramid_disputes_match_the_prefix_closure() {
    let cells: Vec<Vec<u8>> = vec![vec![0, 0], vec![0, 1], vec![1, 0], vec![1, 1]];
    let (a, b) = divergent_cells_pair(&cells, 1, LeafOrder::Outside);
    let a_before = live(&a);
    let b_before = live(&b);
    let a_leads = a_initiates(&a, &b);

    let (ours, theirs, a_stats, b_stats) = mirror_with_stats(a, b);
    assert_eq!(ours, theirs, "endpoints converge");

    let by_depth = expected_disputes_by_depth(&cells);
    assert_eq!(
        by_depth,
        vec![1, 2, 4],
        "closure: root, two arms, four cells"
    );
    let (initiator, responder) = split_by_parity(&by_depth);
    let (expected_a, expected_b) = if a_leads {
        (initiator, responder)
    } else {
        (responder, initiator)
    };
    assert_eq!(a_stats.disputed_scopes, expected_a);
    assert_eq!(b_stats.disputed_scopes, expected_b);
    assert_eq!(
        a_stats.disputed_scopes + b_stats.disputed_scopes,
        7,
        "the pair's counts sum to the session's total disputed scopes",
    );

    // One extra per cell per side: each side gains the other's four.
    assert_eq!(a_stats.messages_gained, 4);
    assert_eq!(b_stats.messages_gained, 4);
    assert_eq!(a_stats.messages_shed, 0);
    assert_eq!(b_stats.messages_shed, 0);
    assert_eq!(live(&ours), a_before + 4);
    assert_eq!(live(&theirs), b_before + 4);

    // The floor window grants one scope at its widest stage, per side.
    assert_eq!(a_stats.window_granted, 1);
    assert_eq!(b_stats.window_granted, 1);
}

/// A deletion honored under a disputed leaf parent is one shed message:
/// the holder gains the survivor and drops its own copy of the forgotten
/// leaf, while the redactor moves nothing.
#[test]
fn honored_redaction_counts_as_shed() {
    let (a, b, expected) = leaf_parent_redaction_pair();
    let (ours, theirs, a_stats, b_stats) = mirror_with_stats(a, b);
    assert_eq!(ours, expected, "the redacted leaf survives nowhere");
    assert_eq!(theirs, expected);

    // `a` held the redacted leaf: it sheds that one copy and gains the
    // survivor `b` inserted. `b` moves nothing: the redacted leaf prunes
    // away before it could be supplied.
    assert_eq!(a_stats.messages_shed, 1);
    assert_eq!(a_stats.messages_gained, 1);
    assert_eq!(b_stats.messages_shed, 0);
    assert_eq!(b_stats.messages_gained, 0);

    // The two leaves are siblings under one 31-byte prefix, so every
    // depth from the root through the leaf parent is jointly occupied
    // and differing: 32 disputed scopes, 16 per side by depth parity.
    assert_eq!(a_stats.disputed_scopes, 16);
    assert_eq!(b_stats.disputed_scopes, 16);
}

/// A leaf-parent only the holder occupies is judged leaf by leaf: the
/// forgotten sibling sheds, the concurrent one crosses, and both sides
/// hold the join oracle's tree in either orientation.
///
/// The counterparty holds nothing under the parent, so no leaf-parent
/// dispute answers for the leaves; the verdict is the streaming filter's
/// own, at leaf height.
#[test]
fn forgotten_sibling_is_judged_at_leaf_height() {
    let (holder, forgetter, expected) = forgotten_sibling_pair();
    assert_eq!(
        join_oracle(holder.clone(), forgetter.clone()),
        expected,
        "the fixture's expectation is the join oracle's"
    );
    for holder_first in [true, false] {
        let (left, right) = if holder_first {
            (holder.clone(), forgetter.clone())
        } else {
            (forgetter.clone(), holder.clone())
        };
        let (ours, theirs, left_stats, right_stats) = mirror_with_stats(left, right);
        assert_eq!(ours, expected, "left side holds the join oracle's tree");
        assert_eq!(theirs, expected, "right side holds the join oracle's tree");
        let (holder_stats, forgetter_stats) = if holder_first {
            (left_stats, right_stats)
        } else {
            (right_stats, left_stats)
        };
        assert_eq!(
            holder_stats.messages_shed, 1,
            "the holder sheds the forgotten leaf"
        );
        assert_eq!(holder_stats.messages_gained, 0);
        assert_eq!(forgetter_stats.messages_shed, 0);
        assert_eq!(
            forgetter_stats.messages_gained, 1,
            "the forgetter gains the concurrent leaf"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// For any forgotten subset of a holder's sibling leaves, both sides
    /// hold the join oracle's tree in either orientation; the holder sheds
    /// exactly the forgotten leaves and the forgetter gains exactly the
    /// rest.
    #[test]
    fn forgotten_siblings_match_the_join_oracle(
        (holder, forgetter, forgotten) in arb_forgotten_siblings(),
        holder_first in any::<bool>(),
    ) {
        let expected = join_oracle(holder.clone(), forgetter.clone());
        let total = live(&holder);
        let (left, right) = if holder_first {
            (holder, forgetter)
        } else {
            (forgetter, holder)
        };
        let (ours, theirs, left_stats, right_stats) = mirror_with_stats(left, right);
        prop_assert_eq!(&ours, &expected);
        prop_assert_eq!(&theirs, &expected);
        let (holder_stats, forgetter_stats) = if holder_first {
            (left_stats, right_stats)
        } else {
            (right_stats, left_stats)
        };
        prop_assert_eq!(holder_stats.messages_shed, forgotten as u64);
        prop_assert_eq!(holder_stats.messages_gained, 0);
        prop_assert_eq!(forgetter_stats.messages_shed, 0);
        prop_assert_eq!(forgetter_stats.messages_gained, total - forgotten as u64);
    }

    /// Across generated antichain corpora, both sides' counters match
    /// the oracle exactly.
    ///
    /// Disputed scopes match the prefix closure per side (by depth
    /// parity) and in sum, each side gains exactly the other's extras
    /// and sheds nothing, and the live count is conserved as
    /// `before + gained - shed`.
    #[test]
    fn dispute_counts_match_the_closure_oracle(cells in arb_cells()) {
        let (a, b) = divergent_cells_pair(&cells, 1, LeafOrder::Outside);
        let a_before = live(&a);
        let b_before = live(&b);
        let equal = a_before == 0 && b_before == 0;
        let a_leads = !equal && a_initiates(&a, &b);

        let (ours, theirs, a_stats, b_stats) = mirror_with_stats(a, b);
        prop_assert_eq!(&ours, &theirs);

        let by_depth = expected_disputes_by_depth(&cells);
        let (initiator, responder) = split_by_parity(&by_depth);
        let (expected_a, expected_b) = if equal {
            (0, 0)
        } else if a_leads {
            (initiator, responder)
        } else {
            (responder, initiator)
        };
        prop_assert_eq!(a_stats.disputed_scopes, expected_a);
        prop_assert_eq!(b_stats.disputed_scopes, expected_b);

        // One extra per cell per side; nothing is causally past the
        // other side, so nothing sheds.
        let extras = cells.len() as u64;
        prop_assert_eq!(a_stats.messages_gained, extras);
        prop_assert_eq!(b_stats.messages_gained, extras);
        prop_assert_eq!(a_stats.messages_shed, 0);
        prop_assert_eq!(b_stats.messages_shed, 0);

        // Conservation: the session moves the live count by exactly
        // gained minus shed.
        prop_assert_eq!(live(&ours), a_before + a_stats.messages_gained);
        prop_assert_eq!(live(&theirs), b_before + b_stats.messages_gained);
    }
}
