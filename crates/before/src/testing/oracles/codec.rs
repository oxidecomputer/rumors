//! The wire formats of versions, parties, and ranks, written out as an
//! encoder that spells any tree, canonical or not, and as a specification
//! of what a strict decoder owes any byte string ([`verdict`]).
//!
//! Production's writers emit only canonical encodings, so they cannot build
//! the inputs a strict decoder exists to reject: a sibling pair that should
//! have collapsed, or a running height below zero. This encoder spells
//! whatever tree it is given, and it shares no code with production.
//!
//! Only the rank format has a full public specification, the `rank` module
//! doc, which [`rank_bits`] transcribes. No public doc states the version or
//! party bit layout: the party layout here follows the private
//! `party::io::writer` module doc, and the version layout (tag polarity,
//! gamma codes, and the zigzag mapping of height changes) follows production's
//! version writer and payload decoder. Production is therefore the de facto
//! authority for those layouts, and this module's tests calibrate its
//! canonical spellings against production's encoders. What the encoder adds
//! is independence where it matters to a decoder test: it spells
//! non-canonical trees, which no production writer can.
//!
//! [`Version::from_oracle`] and [`Party::from_oracle`] read recursive-oracle
//! trees, which a test may lower from production values through the bridge;
//! the encoding itself never consults production.
//!
//! # Flat trees
//!
//! Both tree types here are *flat*: a full binary tree given by its leaves,
//! left to right, each with its depth and its value. A leaf at depth `d`
//! covers an interval of width `2⁻ᵈ` of the unit interval, and consecutive
//! leaves cover consecutive intervals, so a flat tree is also a step function
//! over `[0, 1)`. A depth list describes a full binary tree exactly when a
//! preorder traversal that opens branches down to each listed depth places
//! every leaf and ends with no slot left open ([`preorder`]).
//!
//! Unlike a boxed tree, a flat tree has no recursive structure, so every
//! operation below is a loop, and no depth can exhaust the stack in a
//! traversal or in a drop.
//!
//! # The formats
//!
//! - A *version stream* lists the tree in preorder: a `0` for each branch, a
//!   `1` for each leaf followed by an Elias gamma code. The first leaf's code
//!   is its height plus one; each later leaf's is its zigzag-coded change
//!   from the previous leaf's height (`2d` for a rise `d ≥ 0`, `2|d| - 1`
//!   for a fall), plus one.
//! - A *party stream* lists the tree in preorder, two bits per node: an
//!   owned leaf is `00`; a branch is its two children's presence bits, and a
//!   child whose whole subtree is unowned is absent and spelled by nothing.
//! - Both streams are *sealed* ([`seal`]): a `1` marker bit follows the live
//!   bits, and zero bits fill the last byte.
//! - A *rank stream* ([`rank_bits`]) closes itself with a `0` bit after its
//!   fraction groups, and zero bits fill its last byte ([`pack`]).
//!
//! # Canonical spellings
//!
//! A version's spelling is canonical when its flat tree is normal (no two
//! sibling leaves share a height, [`Version::normalized`]) and no height is
//! negative. A party's spelling is canonical when no two sibling leaves are
//! both owned. An unowned pair has no spelling of its own: an unowned
//! subtree is absent from the stream, so a party with an uncollapsed unowned
//! pair spells exactly what its normal form spells. A rank's spelling is
//! canonical when its numerator is odd or its exponent is zero
//! ([`normalize_rank`]). The encoder spells non-canonical versions, parties,
//! and ranks just as faithfully, which is what the near-valid generators need
//! from it.

use num_bigint::{BigInt, BigUint, Sign};

use crate::testing::oracles::tree;

mod tests;
// The decoder properties read the specification, and the near-valid census
// reads its framing of a span's lower endpoint for their record leg; the
// properties need both codec features, so the census's reading does too.
#[cfg(all(feature = "serde", feature = "borsh"))]
pub(crate) mod verdict;

// ───────────────────────────── bits and bytes ─────────────────────────────

/// Packs bits into bytes, most significant bit first, filling the last byte
/// with zero bits.
pub(crate) fn pack(bits: &[bool]) -> Vec<u8> {
    bits.chunks(8)
        .map(|chunk| {
            chunk
                .iter()
                .enumerate()
                .fold(0u8, |byte, (i, &bit)| byte | (u8::from(bit) << (7 - i)))
        })
        .collect()
}

/// Seals a version or party stream: its live bits, a `1` marker bit, then
/// zero bits to the byte boundary.
pub(crate) fn seal(bits: &[bool]) -> Vec<u8> {
    let mut sealed = bits.to_vec();
    sealed.push(true);
    pack(&sealed)
}

/// Appends the Elias gamma code of `n + 1`: as many `0` bits as `n + 1` has
/// bits after its leading one, then `n + 1` itself.
fn push_gamma(n: &BigUint, out: &mut Vec<bool>) {
    let m = n + 1u32;
    let width = m.bits();
    out.extend(std::iter::repeat_n(false, (width - 1) as usize));
    out.extend((0..width).rev().map(|i| m.bit(i)));
}

/// The zigzag code of a height change: `2d` for a rise `d ≥ 0` and
/// `2|d| - 1` for a fall.
fn zigzag(change: &BigInt) -> BigUint {
    let doubled = change.magnitude() << 1u32;
    if change.sign() == Sign::Minus {
        doubled - 1u32
    } else {
        doubled
    }
}

// ───────────────────────────── flat trees ─────────────────────────────

/// One step of a flat tree's preorder traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// A branch opens; its left subtree's steps follow, then its right's.
    Branch,
    /// The leaf at this index of the flat tree.
    Leaf(usize),
}

/// The preorder steps of the full binary tree whose leaves, left to right,
/// have `depths`, or `None` if no full binary tree has them.
///
/// The traversal keeps the depths of the right children still to be filled.
/// Each leaf fills the next open slot, after opening branches from the
/// slot's depth down to its own; a leaf shallower than its slot, a leaf after
/// the last slot is filled, or a slot left open at the end means the list
/// describes no tree.
fn preorder(depths: impl IntoIterator<Item = u64>) -> Option<Vec<Step>> {
    let mut steps = Vec::new();
    let mut open_right = Vec::new();
    let mut slot = Some(0u64);
    let mut leaves = 0;
    for (index, depth) in depths.into_iter().enumerate() {
        let at = slot?;
        if depth < at {
            return None;
        }
        for level in at..depth {
            steps.push(Step::Branch);
            open_right.push(level + 1);
        }
        steps.push(Step::Leaf(index));
        slot = open_right.pop();
        leaves += 1;
    }
    (leaves > 0 && slot.is_none()).then_some(steps)
}

/// Whether `leaves` describe a full binary tree.
fn is_tree<T>(leaves: &[(u64, T)]) -> bool {
    preorder(leaves.iter().map(|&(depth, _)| depth)).is_some()
}

/// Collapses every pair of sibling leaves with equal values into one leaf at
/// their parent's depth, bottom up, until no such pair remains.
///
/// The traversal rebuilds the tree subtree by subtree. A completed subtree is
/// the run of output leaves from its start index; when a branch completes and
/// both its subtrees are single leaves with equal values, the right leaf is
/// dropped and the left rises to the branch's depth. Collapsing can make the
/// branch's parent collapsible in turn, which the same fold handles as the
/// traversal unwinds.
fn collapse<T: Clone + PartialEq>(leaves: &[(u64, T)]) -> Vec<(u64, T)> {
    enum Frame {
        /// A branch waiting for its left subtree.
        NeedLeft,
        /// A branch waiting for its right subtree; its left subtree starts at
        /// this output index.
        NeedRight(usize),
    }
    let steps = preorder(leaves.iter().map(|&(depth, _)| depth))
        .expect("a flat tree's depths describe a full binary tree");
    let mut out: Vec<(u64, T)> = Vec::with_capacity(leaves.len());
    let mut frames = Vec::new();
    for step in steps {
        let Step::Leaf(index) = step else {
            frames.push(Frame::NeedLeft);
            continue;
        };
        out.push(leaves[index].clone());
        let mut done = out.len() - 1;
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::NeedLeft => {
                    frames.push(Frame::NeedRight(done));
                    break;
                }
                Frame::NeedRight(left) => {
                    let both_leaves = out.len() == left + 2;
                    if both_leaves && out[left].1 == out[left + 1].1 {
                        out.pop();
                        out[left].0 -= 1;
                    }
                    done = left;
                }
            }
        }
    }
    out
}

/// The common refinement of two flat trees: one leaf for each nonempty
/// intersection of a leaf of `a` with a leaf of `b`, carrying both values.
///
/// Positions are counted in units of `2⁻ᵗ` for the deeper tree's depth `t`.
/// Two dyadic intervals that start together are nested, so each step's piece
/// is the narrower of the two current leaves, whole; the wider leaf keeps its
/// remainder for the next step.
fn refine<'a, T, U>(a: &'a [(u64, T)], b: &'a [(u64, U)]) -> Vec<(u64, &'a T, &'a U)> {
    let top = a
        .iter()
        .map(|&(depth, _)| depth)
        .chain(b.iter().map(|&(depth, _)| depth))
        .max()
        .expect("flat trees are nonempty");
    let width = |depth: u64| BigUint::from(1u8) << (top - depth);
    let mut out = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    let (mut rest_a, mut rest_b) = (width(a[0].0), width(b[0].0));
    loop {
        let piece = rest_a.clone().min(rest_b.clone());
        debug_assert_eq!(
            piece.count_ones(),
            1,
            "a refinement piece is one dyadic leaf"
        );
        out.push((top - (piece.bits() - 1), &a[i].1, &b[j].1));
        rest_a -= &piece;
        rest_b -= &piece;
        if rest_b == BigUint::ZERO {
            j += 1;
            if j < b.len() {
                rest_b = width(b[j].0);
            }
        }
        if rest_a == BigUint::ZERO {
            i += 1;
            if i == a.len() {
                break;
            }
            rest_a = width(a[i].0);
        }
    }
    assert_eq!(j, b.len(), "two flat trees cover the same unit interval");
    out
}

// ───────────────────────────── versions ─────────────────────────────

/// A version as a flat tree of absolute leaf heights.
///
/// Heights are signed so that a tree can carry the negative running height
/// a strict decoder must reject. Unlike the recursive oracle's
/// [`tree::Version`], which stores relative bases on every node, a leaf here
/// stores the absolute height the wire format's payloads are differences of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Version {
    /// The leaves, left to right, each as its depth and absolute height.
    leaves: Vec<(u64, BigInt)>,
}

impl Version {
    /// Builds a version from its leaves, left to right.
    ///
    /// # Panics
    ///
    /// Panics if the depths describe no full binary tree.
    pub(crate) fn from_leaves(leaves: Vec<(u64, BigInt)>) -> Self {
        assert!(
            is_tree(&leaves),
            "a flat version's depths must describe a full binary tree"
        );
        Version { leaves }
    }

    /// The leaves, left to right, each as its depth and absolute height.
    pub(crate) fn leaves(&self) -> &[(u64, BigInt)] {
        &self.leaves
    }

    /// The depth of the deepest leaf.
    pub(crate) fn depth(&self) -> u64 {
        self.leaves
            .iter()
            .map(|&(depth, _)| depth)
            .max()
            .unwrap_or(0)
    }

    /// The normal form of the same step function: every sibling pair of equal
    /// heights collapsed into its parent.
    pub(crate) fn normalized(&self) -> Version {
        Version {
            leaves: collapse(&self.leaves),
        }
    }

    /// Combines two step functions pointwise with `f`, in normal form.
    pub(crate) fn zip(&self, other: &Version, f: impl Fn(&BigInt, &BigInt) -> BigInt) -> Version {
        let pieces = refine(&self.leaves, &other.leaves)
            .into_iter()
            .map(|(depth, a, b)| (depth, f(a, b)))
            .collect::<Vec<_>>();
        Version {
            leaves: collapse(&pieces),
        }
    }

    /// Whether this step function lies at or below `other` everywhere: the
    /// versions' causal order.
    pub(crate) fn le(&self, other: &Version) -> bool {
        refine(&self.leaves, &other.leaves)
            .into_iter()
            .all(|(_, a, b)| a <= b)
    }

    /// The area under the step function as a normalized `(num, exp)`, the
    /// rank `num / 2^exp`.
    ///
    /// # Panics
    ///
    /// Panics if a height is negative: ranks are areas of nonnegative
    /// functions.
    pub(crate) fn rank(&self) -> (BigUint, u64) {
        let exp = self.depth();
        let area: BigInt = self
            .leaves
            .iter()
            .map(|(depth, height)| height << (exp - depth))
            .sum();
        let num = area
            .to_biguint()
            .expect("a rank is the area of a nonnegative step function");
        normalize_rank(num, exp)
    }

    /// The version stream's live bits, before sealing.
    ///
    /// # Panics
    ///
    /// Panics if the first leaf's height is negative: the format spells the
    /// first height as an unsigned code, so no stream carries it.
    pub(crate) fn live_bits(&self) -> Vec<bool> {
        let steps = preorder(self.leaves.iter().map(|&(depth, _)| depth))
            .expect("a flat version describes a full binary tree");
        let mut out = Vec::new();
        for step in steps {
            let Step::Leaf(index) = step else {
                out.push(false);
                continue;
            };
            out.push(true);
            let height = &self.leaves[index].1;
            if index == 0 {
                let first = height
                    .to_biguint()
                    .expect("the first leaf's height is spelled unsigned");
                push_gamma(&first, &mut out);
            } else {
                push_gamma(&zigzag(&(height - &self.leaves[index - 1].1)), &mut out);
            }
        }
        out
    }

    /// The sealed version stream: canonical exactly when this version is
    /// normal and no height is negative.
    ///
    /// # Panics
    ///
    /// Panics if the first leaf's height is negative, as
    /// [`live_bits`](Self::live_bits) does.
    pub(crate) fn encode(&self) -> Vec<u8> {
        seal(&self.live_bits())
    }

    /// The flat form of a recursive-oracle version, in which each leaf's
    /// absolute height is the sum of the bases on its path.
    pub(crate) fn from_oracle(version: &tree::Version) -> Version {
        let mut leaves = Vec::new();
        let mut stack = vec![(version, 0u64, BigUint::ZERO)];
        while let Some((node, depth, offset)) = stack.pop() {
            match node {
                tree::Version::Leaf(base) => leaves.push((depth, BigInt::from(offset + base))),
                tree::Version::Node(base, left, right) => {
                    let offset = offset + base;
                    stack.push((right, depth + 1, offset.clone()));
                    stack.push((left, depth + 1, offset));
                }
            }
        }
        Version { leaves }
    }
}

// ───────────────────────────── parties ─────────────────────────────

/// A party as a flat tree of leaf ownership.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Party {
    /// The leaves, left to right, each as its depth and whether it is owned.
    leaves: Vec<(u64, bool)>,
}

impl Party {
    /// Builds a party from its leaves, left to right.
    ///
    /// # Panics
    ///
    /// Panics if the depths describe no full binary tree.
    pub(crate) fn from_leaves(leaves: Vec<(u64, bool)>) -> Self {
        assert!(
            is_tree(&leaves),
            "a flat party's depths must describe a full binary tree"
        );
        Party { leaves }
    }

    /// The leaves, left to right, each as its depth and whether it is owned.
    pub(crate) fn leaves(&self) -> &[(u64, bool)] {
        &self.leaves
    }

    /// The depth of the deepest leaf.
    pub(crate) fn depth(&self) -> u64 {
        self.leaves
            .iter()
            .map(|&(depth, _)| depth)
            .max()
            .unwrap_or(0)
    }

    /// The normal form of the same region: every sibling pair of equal
    /// ownership collapsed into its parent.
    pub(crate) fn normalized(&self) -> Party {
        Party {
            leaves: collapse(&self.leaves),
        }
    }

    /// The party stream's live bits, before sealing.
    ///
    /// The tree is rebuilt as an arena, whose preorder construction numbers
    /// every child after its parent, so one reverse pass finds which subtrees
    /// own anything. The emitting traversal then writes each branch's
    /// presence bits and visits only the present children.
    ///
    /// # Panics
    ///
    /// Panics if no leaf is owned: the format has no spelling for an empty
    /// region.
    pub(crate) fn live_bits(&self) -> Vec<bool> {
        enum Node {
            Branch([usize; 2]),
            Leaf(bool),
        }
        let steps = preorder(self.leaves.iter().map(|&(depth, _)| depth))
            .expect("a flat party describes a full binary tree");
        let mut nodes: Vec<Node> = Vec::with_capacity(steps.len());
        let mut filling: Vec<(usize, usize)> = Vec::new();
        for step in steps {
            let index = nodes.len();
            if let Some((parent, filled)) = filling.last_mut() {
                let Node::Branch(children) = &mut nodes[*parent] else {
                    unreachable!("only branches wait for children");
                };
                children[*filled] = index;
                *filled += 1;
                if *filled == 2 {
                    filling.pop();
                }
            }
            match step {
                Step::Branch => {
                    nodes.push(Node::Branch([0, 0]));
                    filling.push((index, 0));
                }
                Step::Leaf(leaf) => nodes.push(Node::Leaf(self.leaves[leaf].1)),
            }
        }
        let mut owns = vec![false; nodes.len()];
        for index in (0..nodes.len()).rev() {
            owns[index] = match nodes[index] {
                Node::Leaf(owned) => owned,
                Node::Branch([left, right]) => owns[left] || owns[right],
            };
        }
        assert!(owns[0], "the format has no spelling for an empty party");
        let mut out = Vec::new();
        let mut stack = vec![0];
        while let Some(index) = stack.pop() {
            match nodes[index] {
                Node::Leaf(_) => out.extend([false, false]),
                Node::Branch([left, right]) => {
                    out.extend([owns[left], owns[right]]);
                    if owns[right] {
                        stack.push(right);
                    }
                    if owns[left] {
                        stack.push(left);
                    }
                }
            }
        }
        out
    }

    /// The sealed party stream: canonical exactly when no two sibling leaves
    /// are both owned.
    ///
    /// Unowned subtrees are absent from the stream, so an uncollapsed unowned
    /// pair spells the same bytes as its normal form.
    ///
    /// # Panics
    ///
    /// Panics if no leaf is owned, as [`live_bits`](Self::live_bits) does.
    pub(crate) fn encode(&self) -> Vec<u8> {
        seal(&self.live_bits())
    }

    /// The flat form of a recursive-oracle party, normal or not.
    pub(crate) fn from_oracle(party: &tree::Party) -> Party {
        let mut leaves = Vec::new();
        let mut stack = vec![(party, 0u64)];
        while let Some((node, depth)) = stack.pop() {
            match node {
                tree::Party::Leaf(owned) => leaves.push((depth, *owned)),
                tree::Party::Node(left, right) => {
                    stack.push((right, depth + 1));
                    stack.push((left, depth + 1));
                }
            }
        }
        Party { leaves }
    }
}

// ───────────────────────────── ranks ─────────────────────────────

/// Normalizes the rank `num / 2^exp`: cancels the factors of two the
/// numerator and denominator share, and gives zero the exponent zero.
pub(crate) fn normalize_rank(num: BigUint, exp: u64) -> (BigUint, u64) {
    match num.trailing_zeros() {
        None => (BigUint::ZERO, 0),
        Some(zeros) => {
            let shift = zeros.min(exp);
            (num >> shift, exp - shift)
        }
    }
}

/// The rank stream of the normalized rank `num / 2^exp`, through its closing
/// bit.
///
/// With `I` the integral part, `m = I + 1`, `w = bits(m)`, and
/// `rho = bits(w) - 1`, the stream is `rho` ones, a zero, the `rho` low bits
/// of `w`, and the `w - 1` low bits of `m`. The fraction digits follow in
/// groups of eight, each group after a `1` opener and the last zero-filled,
/// and a `0` closes the fraction.
pub(crate) fn rank_bits(num: &BigUint, exp: u64) -> Vec<bool> {
    let m = (num >> exp) + 1u32;
    let w = m.bits();
    let rho = u64::from(u64::BITS - w.leading_zeros()) - 1;
    let mut out = vec![true; rho as usize];
    out.push(false);
    out.extend((0..rho).rev().map(|i| w >> i & 1 == 1));
    out.extend((0..w - 1).rev().map(|i| m.bit(i)));
    for group in 0..exp.div_ceil(8) {
        out.push(true);
        out.extend((group * 8 + 1..=(group + 1) * 8).map(|j| j <= exp && num.bit(exp - j)));
    }
    out.push(false);
    out
}

/// The canonical bytes of the normalized rank `num / 2^exp`.
pub(crate) fn encode_rank(num: &BigUint, exp: u64) -> Vec<u8> {
    pack(&rank_bits(num, exp))
}
