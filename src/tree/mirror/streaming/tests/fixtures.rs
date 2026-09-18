//! Deterministic tree shapes for streaming integration, capacity, and
//! skeleton-bridge tests.

use std::collections::BTreeSet;

use proptest::prelude::*;
use serde::Serialize;

use crate::{
    Version,
    message::Message,
    tree::{
        Root,
        arb::nth_party,
        mirror::streaming::window::FAN,
        traverse::{Action, act},
        typed::{Node as TreeNode, Path, height},
    },
};

/// A 32-byte path with the given prefix, zero-padded.
pub(super) fn path_at(prefix: &[u8]) -> Path {
    let mut bytes = [0u8; 32];
    bytes[..prefix.len()].copy_from_slice(prefix);
    Path::from(bytes)
}

/// Leaf bytes on the all-zero leaf parent, distinguished by the final byte.
pub(super) fn leaf_sibling_bytes(last: u8) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[31] = last;
    bytes
}

/// A leaf path on the all-zero leaf parent, distinguished by its final byte.
pub(super) fn leaf_sibling_path(last: u8) -> Path {
    Path::from(leaf_sibling_bytes(last))
}

/// Extend `node` with one leaf per path, all carrying `value`, versioned on
/// one fresh chain of party `party`, ticking `stride` times per leaf.
///
/// Building successive trees on top of a shared node keeps the shared
/// subtrees hash-identical by construction; distinct parties keep each
/// side's extras causally concurrent, so nothing is deletion-pruned when
/// provided across. The `stride` and `value` parameters perturb payloads:
/// they change versions or contents (hence every hash on the path) without
/// touching the path structure the reconciliation keys on.
pub(super) fn grown<T>(
    node: Option<TreeNode<height::Root>>,
    party: usize,
    stride: usize,
    value: &T,
    paths: &[Path],
) -> Option<TreeNode<height::Root>>
where
    T: Serialize + Clone + Send + Sync + 'static,
{
    assert!(stride > 0, "each leaf needs a fresh version");
    let party = nth_party(party);
    let mut version = Version::new();
    let mut actions = Vec::new();
    for path in paths {
        for _ in 0..stride {
            version.tick(&party);
        }
        actions.push((
            *path,
            version.clone(),
            Action::Insert(Message::new(value.clone())),
        ));
    }
    act(node, actions, &mut |_| ())
}

/// Wrap a node as a [`Root`] whose ceiling is the node's own.
pub(super) fn rooted(node: Option<TreeNode<height::Root>>) -> Root {
    Root {
        ceiling: ceiling_of(&node),
        node,
    }
}

/// Wrap a node as a [`Root`] advertising an explicit ceiling.
///
/// Used to equalize two remote trees' greeting versions so role election
/// (`streaming.rs::descend`: the smaller set initiates, canonical version
/// bytes break ties) comes out identical across two sessions against the
/// same local tree once the remotes' set sizes are also equal. Inflating a
/// ceiling with ticks from parties absent from the tree's leaves is inert here:
/// deletion pruning compares leaf versions with the counterparty's ceiling,
/// and each fixture keeps supplied leaves on chains that ceiling never covers.
pub(super) fn rooted_at(node: Option<TreeNode<height::Root>>, ceiling: Version) -> Root {
    Root { ceiling, node }
}

/// The ceiling a node would advertise on its own.
pub(super) fn ceiling_of(node: &Option<TreeNode<height::Root>>) -> Version {
    node.as_ref()
        .map(TreeNode::ceiling)
        .cloned()
        .unwrap_or_default()
}

// ------------------------------------------------- generated divergence specs

/// The version-chain party index of each tree's leaves in a [`Divergence`].
///
/// Parties are pairwise disjoint, so every side's extras stay causally
/// concurrent with everything else and survive deletion-pruning when
/// provided across (crate safety rules: one universe, linear parties).
const SHARED_PARTY: usize = 0;
/// The first remote's extras chain.
const REMOTE_0_PARTY: usize = 1;
/// The local tree's extras chain.
const LOCAL_PARTY: usize = 2;
/// The second remote's extras chain.
const REMOTE_1_PARTY: usize = 3;

/// The last-path-byte namespace of each leaf population under a cell.
///
/// Cell prefix bytes stay below `0x10`, so a cell can never collide with
/// another cell's slot column.
const SHARED_SLOT: u8 = 0x10;
/// The local tree's one-sided extra under a cell.
const LOCAL_SLOT: u8 = 0x40;
/// The remotes' one-sided extras under a cell.
const REMOTE_SLOTS: [u8; 2] = [0x80, 0xc0];

/// One divergence cell: a controlled prefix and the leaf populations under
/// it.
///
/// Every difference is one-sided (distinct slot columns per population), so
/// no key is ever present on both sides with different content — the shape
/// `answer::leaf_parent`'s unconditional `Both -> Match` makes meaningless.
/// A `deep` cell is zero-extended to 31 bytes, so its populations share a
/// leaf parent and the dispute descends the full trie: that is what
/// exercises height-1 scopes and leaf requests.
#[derive(Clone, Debug)]
pub(super) struct CellSpec {
    /// The controlled prefix (bytes < 0x10, length <= 3).
    pub cell: Vec<u8>,
    /// Zero-extend the cell to a full 31-byte leaf parent.
    pub deep: bool,
    /// How many shared leaves (identical on all sides) sit under the cell.
    pub shared: u8,
    /// Does the local tree hold a one-sided extra here?
    pub local: bool,
    /// Does each remote hold a one-sided extra here?
    pub remote: [bool; 2],
}

/// A generated three-tree divergence: one local tree against two remotes.
#[derive(Clone, Debug)]
pub(super) struct Divergence {
    /// The divergence cells.
    pub cells: Vec<CellSpec>,
}

impl CellSpec {
    /// The full 32-byte path of this cell's leaf in slot column `slot`,
    /// offset by `index` within the column.
    fn path(&self, slot: u8, index: u8) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[..self.cell.len()].copy_from_slice(&self.cell);
        let at = if self.deep { 31 } else { self.cell.len() };
        bytes[at] = slot + index;
        bytes
    }
}

impl Divergence {
    /// The shared leaves' paths, in cell order, first occurrence per path:
    /// sampled cells may repeat a prefix, and one path is one leaf — an
    /// insert never lands on an occupied path.
    pub fn shared_paths(&self) -> Vec<[u8; 32]> {
        let mut seen = BTreeSet::new();
        self.cells
            .iter()
            .flat_map(|cell| (0..cell.shared).map(|i| cell.path(SHARED_SLOT, i)))
            .filter(|path| seen.insert(*path))
            .collect()
    }

    /// The local tree's one-sided extras, deduplicated as
    /// [`shared_paths`](Self::shared_paths) does.
    pub fn local_paths(&self) -> Vec<[u8; 32]> {
        let mut seen = BTreeSet::new();
        self.cells
            .iter()
            .filter(|cell| cell.local)
            .map(|cell| cell.path(LOCAL_SLOT, 0))
            .filter(|path| seen.insert(*path))
            .collect()
    }

    /// Remote `which`'s one-sided extras, padded so both remotes hold the
    /// same leaf count.
    ///
    /// Sampled cells may repeat a prefix, collapsing their extras onto one
    /// leaf, so the count that matters is *distinct* paths. The pad leaves
    /// continue the remote's own slot column under the guaranteed anchor
    /// cell (the empty prefix), so they collide with no other population.
    /// Equal counts keep the two remotes' advertised set sizes identical:
    /// with the shared join ceiling ([`Self::trees`]), the local tree's
    /// role election is then identical across the two sessions.
    pub fn remote_paths(&self, which: usize) -> Vec<[u8; 32]> {
        let distinct = |which: usize| -> BTreeSet<[u8; 32]> {
            self.cells
                .iter()
                .filter(|cell| cell.remote[which])
                .map(|cell| cell.path(REMOTE_SLOTS[which], 0))
                .collect()
        };
        let mine = distinct(which);
        let pad = distinct(1 - which).len().saturating_sub(mine.len());
        let anchor = &self.cells[0];
        mine.into_iter()
            .chain((0..pad).map(|i| {
                anchor.path(
                    REMOTE_SLOTS[which],
                    1 + u8::try_from(i).expect("pad counts stay within a slot column"),
                )
            }))
            .collect()
    }

    /// Every path the local tree holds: the membership oracle the view
    /// soundness checks compare skeleton scopes against.
    pub fn local_path_set(&self) -> BTreeSet<[u8; 32]> {
        self.shared_paths()
            .into_iter()
            .chain(self.local_paths())
            .collect()
    }

    /// Build `(local, remote_0, remote_1)`, every leaf carrying `value`.
    ///
    /// All three grow from one shared base node, so shared subtrees are
    /// hash-identical by construction. Both remotes advertise the JOIN of
    /// their natural ceilings ([`rooted_at`]'s inertness argument) and hold
    /// equal leaf counts ([`Self::remote_paths`]' padding), so both of the
    /// role election's keys — set size first, version bytes on ties — are
    /// identical across the two sessions and the local tree plays one role
    /// in both.
    pub fn trees<T>(&self, value: &T) -> (Root, Root, Root)
    where
        T: Serialize + Clone + Send + Sync + 'static,
    {
        let as_paths =
            |bytes: Vec<[u8; 32]>| -> Vec<Path> { bytes.into_iter().map(Path::from).collect() };
        let base = grown(None, SHARED_PARTY, 1, value, &as_paths(self.shared_paths()));
        let local = grown(
            base.clone(),
            LOCAL_PARTY,
            1,
            value,
            &as_paths(self.local_paths()),
        );
        let remote_0 = grown(
            base.clone(),
            REMOTE_0_PARTY,
            1,
            value,
            &as_paths(self.remote_paths(0)),
        );
        let remote_1 = grown(
            base,
            REMOTE_1_PARTY,
            1,
            value,
            &as_paths(self.remote_paths(1)),
        );
        let join = ceiling_of(&remote_0) | &ceiling_of(&remote_1);
        (
            rooted(local),
            rooted_at(remote_0, join.clone()),
            rooted_at(remote_1, join),
        )
    }
}

/// Generate a [`Divergence`].
///
/// The first cell is a fixed anchor at the root with a shared leaf and every
/// one-sided extra: it guarantees nonempty trees, guarantees the local tree
/// differs from both remotes (no equal-version short circuit), and puts at
/// least one tick on every party's chain (distinct ceilings for role
/// election). The rest is random shape: shallow and deep cells, one-sided
/// extras in any combination.
pub(super) fn arb_divergence() -> impl Strategy<Value = Divergence> {
    let cell = (
        proptest::collection::vec(0u8..4, 0..=3),
        any::<bool>(),
        0u8..=2,
        any::<bool>(),
        any::<[bool; 2]>(),
    )
        .prop_map(|(cell, deep, shared, local, remote)| CellSpec {
            cell,
            deep,
            shared,
            local,
            remote,
        });
    proptest::collection::vec(cell, 0..=4).prop_map(|mut cells| {
        cells.insert(
            0,
            CellSpec {
                cell: Vec::new(),
                deep: false,
                shared: 1,
                local: true,
                remote: [true, true],
            },
        );
        Divergence { cells }
    })
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
pub(super) fn one_sided_pair(spec: &[(u8, usize, usize)]) -> (Root, Root) {
    let path = |b0: u8, b1: u8| {
        let mut bytes = [0u8; 32];
        bytes[0] = b0;
        bytes[1] = b1;
        Path::from(bytes)
    };

    assert!(
        spec.iter().all(|&(_, shared, extra)| shared + extra <= FAN),
        "shared and extra leaves must fit one radix fan"
    );
    let shared = spec
        .iter()
        .flat_map(|&(radix, count, _)| {
            (0..count)
                .map(move |index| path(radix, u8::try_from(index).expect("a radix fan fits in u8")))
        })
        .collect::<Vec<_>>();
    let extras = spec
        .iter()
        .flat_map(|&(radix, _, count)| {
            (0..count).map(move |index| {
                path(
                    radix,
                    u8::MAX - u8::try_from(index).expect("a radix fan fits in u8"),
                )
            })
        })
        .collect::<Vec<_>>();

    // Both trees share party 0's base. Party 1's extras remain concurrent
    // with that base and therefore survive when supplied to the first tree.
    let a = grown(None, 0, 1, &(), &shared);
    let b = grown(a.clone(), 1, 1, &(), &extras);
    (rooted(a), rooted(b))
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
    /// Largest shared run when the two extras occupy the end slots.
    const OUTSIDE_SHARED_LIMIT: usize = FAN - 2;

    /// Largest shared run when even slots hold it and odd slots hold extras.
    const INTERLEAVED_SHARED_LIMIT: usize = (FAN - 1) / 2;

    /// Assign distinct leaf slots to the shared run and both extras.
    fn slots(self, shared: usize) -> (Vec<u8>, u8, u8) {
        let slots = |step: usize| {
            (1..=shared)
                .map(|slot| u8::try_from(slot * step).expect("shared slots fit one radix fan"))
                .collect()
        };
        match self {
            Self::Outside | Self::Reversed => {
                assert!((1..=Self::OUTSIDE_SHARED_LIMIT).contains(&shared));
                let (a, b) = if matches!(self, Self::Outside) {
                    (u8::MIN, u8::MAX)
                } else {
                    (u8::MAX, u8::MIN)
                };
                (slots(1), a, b)
            }
            Self::Interleaved => {
                assert!((1..=Self::INTERLEAVED_SHARED_LIMIT).contains(&shared));
                (slots(2), 0x03, 0x01)
            }
        }
    }
}

/// Build a bidirectionally divergent pair over explicitly chosen prefix cells.
pub(super) fn divergent_cells_pair(
    cells: &[Vec<u8>],
    shared: usize,
    order: LeafOrder,
) -> (Root, Root) {
    assert!(cells.iter().all(|cell| cell.len() < 32));
    let (shared_slots, a_slot, b_slot) = order.slots(shared);
    let path = |cell: &[u8], slot: u8| {
        let mut bytes = [0u8; 32];
        bytes[..cell.len()].copy_from_slice(cell);
        bytes[cell.len()] = slot;
        Path::from(bytes)
    };

    let base_paths = cells
        .iter()
        .flat_map(|cell| shared_slots.iter().map(|&slot| path(cell, slot)))
        .collect::<Vec<_>>();
    let extra_paths = |slot| {
        cells
            .iter()
            .map(|cell| path(cell, slot))
            .collect::<Vec<_>>()
    };

    // The trees share party 0's base. Their extras use disjoint parties, so
    // both sets remain concurrent and survive reconciliation.
    let base = grown(None, 0, 1, &(), &base_paths);
    let a = grown(base.clone(), 2, 1, &(), &extra_paths(a_slot));
    let b = grown(base, 1, 1, &(), &extra_paths(b_slot));
    (rooted(a), rooted(b))
}

/// Build a cartesian pyramid whose disputes descend every controlled level.
pub(super) fn pyramid_pair(widths: &[usize], shared: usize, order: LeafOrder) -> (Root, Root) {
    assert!(widths.iter().all(|&width| (1..=FAN).contains(&width)));
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
pub(super) fn full_depth_comb_pair(shared: usize, order: LeafOrder) -> (Root, Root) {
    let mut cells = vec![vec![0; 31]];
    for depth in 0..31 {
        let mut cell = vec![0; 31];
        cell[depth] = 1;
        cells.push(cell);
    }
    divergent_cells_pair(&cells, shared, order)
}
