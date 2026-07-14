//! Deterministic tree shapes for streaming integration and capacity tests.

use crate::{
    Version,
    message::Message,
    tree::{
        Root,
        arb::nth_party,
        traverse::{Action, act},
        typed::{Node as TreeNode, Path, height},
    },
};

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
pub(super) fn one_sided_pair(spec: &[(u8, u8, u8)]) -> (Root<()>, Root<()>) {
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
pub(super) enum LeafOrder {
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
pub(super) fn divergent_cells_pair(
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
pub(super) fn pyramid_pair(
    widths: &[usize],
    shared: usize,
    order: LeafOrder,
) -> (Root<()>, Root<()>) {
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
pub(super) fn full_depth_comb_pair(shared: usize, order: LeafOrder) -> (Root<()>, Root<()>) {
    let mut cells = vec![vec![0; 31]];
    for depth in 0..31 {
        let mut cell = vec![0; 31];
        cell[depth] = 1;
        cells.push(cell);
    }
    divergent_cells_pair(&cells, shared, order)
}
