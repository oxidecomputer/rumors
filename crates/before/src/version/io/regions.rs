//! Forward traversal of canonical version trees.
//!
//! [`VersionRegionReader`] owns a complete-stream traversal for operations that overlay
//! several versions. [`VersionSubtreeReader`] supplies the same iterative descent and
//! backtracking for a subtree read through a reader owned by its caller. Both
//! retain one direction bit per open ancestor and use the bit reader's word-sized
//! reads for topology runs and payloads.
//!
//! The subtree reader can also summarize a range by its net height change and
//! minimum. This lets ticking skip a region in one scan while preserving the
//! state that a leaf-by-leaf walk would have produced.

use core::cmp::Ordering;

use suanpan::Accumulator;

use num_bigint::{BigInt, BigUint, Sign};

use super::tree::VersionTreeReader;
use crate::accumulator::BigIntAccumulator as _;
use crate::bits::stack::BitStack;
use crate::Version;

mod summary;

pub use summary::{Direction, Extremum, RegionSkip};

/// A forward reader over one dyadic partition.
///
/// Stepping crosses the current region's right boundary and returns the value
/// carried by that boundary. Overlay algorithms decide which readers advance;
/// each domain reader defines what a crossing means.
pub trait RegionReader {
    /// The value carried across one boundary.
    type Crossing;

    /// The current region's depth.
    fn depth(&self) -> u64;

    /// Whether the current region reaches the unit interval's right edge.
    fn done(&self) -> bool;

    /// Advance to the next region and return its crossing value.
    ///
    /// The returned depth is the path depth immediately after the walk turns
    /// from a left subtree into its right sibling, before it descends to the
    /// next region. Calling this after [`done`](Self::done) is an error.
    fn step(&mut self) -> (u64, Self::Crossing);
}

/// A leaf-to-leaf height change.
pub type HeightChange = BigInt;

/// Meaning of the next Version payload.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PayloadKind {
    /// The stream's first payload is an absolute height.
    Absolute,
    /// Every later payload is a signed change from the previous height.
    Delta,
}

impl PayloadKind {
    /// Decode a payload according to its position in the stream.
    pub fn decode(self, code: BigUint) -> BigInt {
        match self {
            PayloadKind::Absolute => BigInt::from(code),
            PayloadKind::Delta => {
                if code.bit(0) {
                    BigInt::from_biguint(Sign::Minus, (code + 1u32) >> 1u32)
                } else {
                    BigInt::from_biguint(Sign::Plus, code >> 1u32)
                }
            }
        }
    }
}

/// A reader over the constant-height regions of one version.
///
/// It retains one direction bit per open ancestor and decodes each topology
/// flag and payload once. The next boundary depth is cached when the reader
/// reaches a leaf, so overlay comparisons do not rescan the path.
pub struct VersionRegionReader<'a> {
    /// The next unread topology flag or payload.
    tree: VersionTreeReader<'a>,
    /// Root-to-current-leaf directions.
    path: BitStack,
    /// Depth to which the next boundary ascends.
    next_flip: u64,
    /// End of the complete version stream.
    len: u64,
}

impl<'a> VersionRegionReader<'a> {
    /// Open a version at its first leaf and return that leaf's absolute height.
    pub fn open(version: &'a Version) -> (Self, BigUint) {
        let mut this = VersionRegionReader {
            tree: VersionTreeReader::new(version),
            path: BitStack::new(),
            next_flip: 0,
            len: version.stored_len(),
        };
        let first = this.descend();
        (this, first)
    }

    /// Find the maximum leaf depth without decoding payloads.
    pub fn max_depth(version: &Version) -> u64 {
        let mut reader = VersionTreeReader::new(version);
        let mut deepest = 0u64;
        let mut walk = VersionSubtreeReader::new();
        while let Some(depth) = walk.descend(&mut reader) {
            deepest = deepest.max(depth);
            reader.skip_payload();
        }
        deepest
    }

    /// The next boundary's depth, without advancing; zero at exhaustion.
    pub fn peek_flip(&self) -> u64 {
        self.next_flip
    }

    /// Fold consecutive boundaries deeper than `bound` into `net`.
    pub fn skip_deeper(&mut self, bound: u64, net: &mut Accumulator) {
        while self.peek_flip() > bound {
            let (_, step) = self.step();
            net.add_bigint(&step);
        }
    }

    /// Descend to the next leaf and return its undecoded payload.
    fn descend(&mut self) -> BigUint {
        let internal_nodes = self.tree.descend_left();
        for _ in 0..internal_nodes {
            self.path.push(false);
        }
        let code = self.tree.payload();
        self.next_flip = self.path.len() - self.path.trailing_ones();
        code
    }
}

impl RegionReader for VersionRegionReader<'_> {
    type Crossing = HeightChange;

    fn depth(&self) -> u64 {
        self.path.len()
    }

    fn done(&self) -> bool {
        self.tree.position() == self.len
    }

    fn step(&mut self) -> (u64, HeightChange) {
        let flip = self.next_flip;
        loop {
            match self.path.pop() {
                Some(true) => continue,
                Some(false) => break,
                None => unreachable!("a final version leaf is never advanced"),
            }
        }
        self.path.push(true);
        debug_assert_eq!(
            self.path.len(),
            flip,
            "the cached boundary matches the path"
        );
        let code = self.descend();
        (flip, PayloadKind::Delta.decode(code))
    }
}

/// The topology walk over one Version subtree's leaves, in preorder.
///
/// The driver reads only topology bits; the payload code at each yielded leaf
/// is the caller's. The cursor is a per-call argument rather than owned state
/// so the caller keeps it between calls — the consuming walks read their
/// payloads through the same cursor the driver descends with, and their
/// surrounding state (range-minimum stacks, output builders, height accumulators)
/// borrows freely alongside.
pub struct VersionSubtreeReader {
    /// Root-to-leaf branch directions for the current leaf, root first: `false`
    /// inside an ancestor's left child (its right subtree is still pending in
    /// the stream), `true` inside its right.
    path: BitStack,
    /// Whether a leaf has been yielded: the first descent has no finished leaf
    /// to backtrack from.
    started: bool,
}

impl VersionSubtreeReader {
    /// A walk positioned to enter the subtree at the caller's cursor.
    pub fn new() -> Self {
        VersionSubtreeReader {
            path: BitStack::new(),
            started: false,
        }
    }

    /// Advance to the next leaf, returning its depth below the walked subtree's
    /// root — or `None` when the previous leaf was the subtree's last.
    ///
    /// Each call closes the ancestors the previous leaf completed (the pop-flip
    /// backtrack), then descends to the leaf at the cursor.
    ///
    /// Between calls the caller must advance the cursor past exactly the
    /// yielded leaf's payload code (`read_gamma` or `skip_gamma`): the driver reads
    /// topology only, and the next descent starts at the following node's flag.
    /// The backtrack is pure path bookkeeping — it reads no bits — so a caller
    /// that stops mid-subtree (a position-bounded prefix pass) simply stops
    /// calling.
    ///
    /// # Panics
    ///
    /// The stream must be canonical. The violations this walk structurally
    /// notices — truncation, malformation — panic; the rest walk silently
    /// with an unspecified result (the contract of
    /// [`Version::partial_cmp`], stated once there).
    pub fn descend(&mut self, cursor: &mut VersionTreeReader<'_>) -> Option<u64> {
        if self.started {
            loop {
                match self.path.pop() {
                    Some(true) => continue,
                    Some(false) => {
                        self.path.push(true);
                        break;
                    }
                    None => return None,
                }
            }
        }
        self.started = true;
        // One whole descent per unary read: the run's internal nodes, then
        // the leaf whose flag terminates the run.
        let internal_nodes = cursor.descend_left();
        for _ in 0..internal_nodes {
            self.path.push(false);
        }
        Some(self.path.len())
    }
}

impl Extremum {
    /// Track the maximum, resetting when the height rises past it.
    ///
    /// Resets keep the register's allocation for later rises within the same
    /// scan. This can avoid repeatedly allocating when several wide maxima are
    /// established.
    pub fn max(register: Accumulator) -> Self {
        Extremum {
            register,
            armed: false,
            direction: Direction::Max,
        }
    }

    /// Track the minimum, resetting when the height drops past it.
    ///
    /// Resets replace the register in O(1), releasing any wide allocation that
    /// described the old minimum instead of scanning it merely to write zeros.
    pub fn min(register: Accumulator) -> Self {
        Extremum {
            register,
            armed: false,
            direction: Direction::Min,
        }
    }

    /// Fold one consumed leaf-to-leaf step.
    ///
    /// The first call establishes the starting leaf and therefore has no
    /// transition to fold.
    pub fn fold(&mut self, delta: &BigInt) {
        if !self.armed {
            self.armed = true;
            return;
        }
        self.fold_armed(delta);
    }

    fn fold_armed(&mut self, delta: &BigInt) {
        self.register.sub_bigint(delta);
        let overtaken = match self.direction {
            Direction::Max => Ordering::Less,
            Direction::Min => Ordering::Greater,
        };
        if self.register.cmp_zero() == overtaken {
            match self.direction {
                Direction::Max => self.register.reset(),
                Direction::Min => self.register = Accumulator::new(),
            }
        }
    }

    /// The finished register, `extremum − h` at the walk's exit.
    pub fn into_offset(self) -> Accumulator {
        self.register
    }
}

impl VersionSubtreeReader {
    /// Fold every remaining payload into `net` and `extremum`.
    ///
    /// Returns the final leaf's depth and payload length. `pending` supplies a
    /// leaf already reached by the caller, avoiding a repeated descent.
    pub fn fold_remaining(
        &mut self,
        cursor: &mut VersionTreeReader<'_>,
        first: PayloadKind,
        net: &mut Accumulator,
        extremum: &mut Extremum,
        pending: Option<u64>,
    ) -> Option<(u64, u64)> {
        let mut kind = first;
        let mut last = None;
        let mut pending = pending;
        loop {
            let depth = match pending.take() {
                Some(depth) => depth,
                None => match self.descend(cursor) {
                    Some(depth) => depth,
                    None => break,
                },
            };
            let start = cursor.position();
            let code = cursor.payload();
            let delta = kind.decode(code);
            kind = PayloadKind::Delta;
            net.add_bigint(&delta);
            extremum.fold(&delta);
            last = Some((depth, cursor.position() - start));
        }
        last
    }

    /// Fold the remaining leaf deltas into their net height change.
    ///
    /// The caller has already descended to the first leaf and leaves its
    /// payload waiting at `cursor`. That pending payload must be delta-coded;
    /// the stream's absolute opening payload must already have been consumed.
    pub fn net_remaining(&mut self, cursor: &mut VersionTreeReader<'_>) -> Accumulator {
        let mut net = Accumulator::new();
        loop {
            let code = cursor.payload();
            let delta = PayloadKind::Delta.decode(code);
            net.add_bigint(&delta);
            if self.descend(cursor).is_none() {
                break;
            }
        }
        net
    }

    /// Summarize the remaining leaves by their net change and minimum.
    ///
    /// `pending` supplies a leaf already reached by the caller, avoiding a
    /// repeated descent. `first` says that leaf carries the stream's absolute
    /// opening height rather than a delta.
    pub fn summarize_remaining(
        &mut self,
        cursor: &mut VersionTreeReader<'_>,
        first: PayloadKind,
        pending: Option<u64>,
    ) -> Option<RegionSkip> {
        let mut net = Accumulator::new();
        let mut min = Extremum::min(Accumulator::new());
        let (last_depth, last_code_len) =
            self.fold_remaining(cursor, first, &mut net, &mut min, pending)?;
        let net = net.to_bigint();
        let min = min.into_offset();
        let min_from_exit = min.to_bigint();
        debug_assert_ne!(
            min_from_exit.sign(),
            Sign::Plus,
            "the minimum is at or below the exit height"
        );
        Some(RegionSkip {
            net,
            min_from_exit,
            last_depth,
            last_code_len,
        })
    }
}

impl RegionSkip {
    /// Summarize the complete subtree at `cursor` in one forward scan.
    ///
    /// The caller has already consumed the stream's opening payload, so the
    /// subtree begins with a leaf-to-leaf delta.
    pub fn read_subtree(cursor: &mut VersionTreeReader<'_>) -> Self {
        let mut walk = VersionSubtreeReader::new();
        // The raise scan calls this after consuming the stream's opening
        // payload, so this subtree begins with a leaf-to-leaf delta.
        walk.summarize_remaining(cursor, PayloadKind::Delta, None)
            .expect("a subtree has at least one leaf")
    }
}
