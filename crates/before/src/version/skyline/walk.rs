//! The leaf-walk driver: one home for the iterative descend-to-leaf/backtrack
//! skeleton every in-order leaf pass over a skyline subtree runs.
//!
//! A skyline subtree's leaves are visited by alternating one word-parallel
//! unary read (the descent: a run of internal flags ended by the leaf's `1`)
//! with a pop-flip backtrack over the root-to-leaf path bits (closing the
//! ancestors the consumed leaf completed). The driver owns exactly that
//! skeleton; everything a pass *does* at a leaf — decode the payload, skip it
//! by width, fold an extremum, emit — stays at the call site, on the caller's
//! own state. Two leaf actions are shared widely enough to live here too:
//! [`Extremum`], the armed, reset-on-overtake streaming max/min the scanning
//! walks fold, and [`skip_region`], the ownership-gated walks' block scan over
//! a subtree none of whose leaves the consumer will touch individually.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::codec::{BitCursor, BitStack, DsiCursor, Int};

use super::signed::{fold_signed_int, unzigzag, Signed};

/// The topology walk over one skyline subtree's leaves, in preorder.
///
/// The driver reads only topology bits; the payload code at each yielded leaf
/// is the caller's. The cursor is a per-call argument rather than owned state
/// so the caller keeps it between calls — the consuming walks read their
/// payloads through the same cursor the driver descends with, and their
/// surrounding state (watermark webs, output builders, height accumulators)
/// borrows freely alongside.
pub(super) struct LeafWalk {
    /// Root-to-leaf branch directions for the current leaf, root first: `false`
    /// inside an ancestor's left child (its right subtree is still pending in
    /// the stream), `true` inside its right.
    path: BitStack,
    /// Whether a leaf has been yielded: the first descent has no finished leaf
    /// to backtrack from.
    started: bool,
}

impl LeafWalk {
    /// A walk positioned to enter the subtree at the caller's cursor.
    pub(super) fn new() -> Self {
        LeafWalk {
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
    /// yielded leaf's payload code (`read_int` or `skip_int`): the driver reads
    /// topology only, and the next descent starts at the following node's flag.
    /// The backtrack is pure path bookkeeping — it reads no bits — so a caller
    /// that stops mid-subtree (a position-bounded prefix pass) simply stops
    /// calling.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    pub(super) fn descend(&mut self, cursor: &mut DsiCursor<'_>) -> Option<usize> {
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
        let internal_nodes = cursor.read_unary().expect("canonical skyline bits");
        for _ in 0..internal_nodes {
            self.path.push(false);
        }
        Some(self.path.len())
    }
}

/// Which extreme an [`Extremum`] tracks.
enum Direction {
    /// The maximum leaf height.
    Max,
    /// The minimum leaf height.
    Min,
}

/// A streaming extremum of the leaf heights a walk consumes, carried relative
/// to the walk's own running height.
///
/// The register holds `extremum − h`: each leaf-to-leaf step folds in
/// *reversed* (the height moved, the extremum did not), and when the register's
/// sign shows the height just crossed the tracked extreme, the register resets
/// to zero — the extremum is the current height again. The first leaf *arms*
/// the fold and is never folded: its payload is the range's entry height, not a
/// leaf-to-leaf movement, so the register starts at zero on it (extremum = h)
/// whatever its coding. The finished offset's width is bounded by the scanned
/// range's own content, which prices every later fold of it.
pub(super) struct Extremum {
    /// `extremum − h`, the running register.
    register: Accumulator,
    /// Whether the first leaf has armed the fold.
    armed: bool,
    /// The extreme the register tracks.
    direction: Direction,
}

impl Extremum {
    /// Track the maximum, resetting when the height rises past it.
    ///
    /// `register` is a leased watermark-pool buffer (zero, returned to its pool
    /// by the caller's materialize), so resets re-zero it in place and the pool
    /// stays warm.
    pub(super) fn max(register: Accumulator) -> Self {
        Extremum {
            register,
            armed: false,
            direction: Direction::Max,
        }
    }

    /// Track the minimum, resetting when the height drops past it.
    ///
    /// `register` is an owned buffer: resets replace it whole, because an
    /// in-place clear scans (and meters) every dead digit a wide swing left
    /// behind where dropping the buffer is O(1).
    pub(super) fn min(register: Accumulator) -> Self {
        Extremum {
            register,
            armed: false,
            direction: Direction::Min,
        }
    }

    /// Fold one consumed leaf-to-leaf step; the arming first call
    /// folds nothing.
    pub(super) fn fold(&mut self, negative: bool, magnitude: &Int) {
        if !self.armed {
            self.armed = true;
            return;
        }
        self.fold_armed(negative, magnitude);
    }

    /// Fold one undecoded zigzag payload code; the arming first call folds
    /// nothing and leaves its code undecoded (an armed leaf's absolute-vs-delta
    /// coding is irrelevant — it is never folded).
    pub(super) fn fold_zigzag(&mut self, code: Int) {
        if !self.armed {
            self.armed = true;
            return;
        }
        let (negative, magnitude) = unzigzag(code);
        self.fold_armed(negative, &magnitude);
    }

    fn fold_armed(&mut self, negative: bool, magnitude: &Int) {
        fold_signed_int(&mut self.register, !negative, magnitude);
        let overtaken = match self.direction {
            Direction::Max => Ordering::Less,
            Direction::Min => Ordering::Greater,
        };
        if self.register.sign() == overtaken {
            match self.direction {
                Direction::Max => self.register.reset(),
                Direction::Min => self.register = Accumulator::new(),
            }
        }
    }

    /// The finished register, `extremum − h` at the walk's exit.
    pub(super) fn into_offset(self) -> Accumulator {
        self.register
    }
}

/// The block summary of one skipped leaf range: the re-entry state a consumer
/// folds to continue exactly as if it had visited the leaves one by one.
///
/// A range the consumer's party has no stake in contributes only two quantities
/// to any ownership-gated pass: where the height ends up, and how low it got on
/// the way. Both arrive as signed magnitudes in the walks' exchange currency,
/// ready to fold into height-carried accumulators and to record as one
/// watermark emission. The last leaf's coordinates ride along for the emitters
/// that splice the range's bits verbatim.
pub(super) struct RegionSkip {
    /// The range's net signed height movement: `h(exit) − h(entry)`.
    pub(super) net: Signed,
    /// The range's minimum leaf height relative to the exit height:
    /// `min − h(exit)`, nonpositive (the exit height is the last
    /// leaf's, itself in the minimum's range).
    pub(super) min_from_exit: Signed,
    /// The last leaf's depth below the walked subtree's root.
    pub(super) last_depth: usize,
    /// The last leaf's payload code length in bits.
    pub(super) last_code_len: usize,
}

/// Drive `walk` over the remaining leaves of the subtree at the cursor, folding
/// every payload into `net` and `extremum` — the block scans' shared read loop.
///
/// One unary topology read and one payload decode per leaf, no other work.
/// Returns the last consumed leaf's depth below the walked root and its code
/// length, or `None` when no leaf remained. `first` says whether the next
/// payload is the stream's absolute first (coded as a height, not a delta);
/// `pending` hands in a leaf the caller has already descended to (its payload
/// still unread at the cursor), which lets a caller route on the first
/// descent's depth without re-reading any bit.
///
/// Every folded bit is still read and recorded: the scan meter's reading is
/// identical to the leaf-by-leaf pass this batches.
///
/// # Panics
///
/// Panics if the stream is not a canonical skyline encoding.
pub(super) fn fold_region(
    walk: &mut LeafWalk,
    cursor: &mut DsiCursor<'_>,
    first: bool,
    net: &mut Accumulator,
    extremum: &mut Extremum,
    pending: Option<usize>,
) -> Option<(usize, usize)> {
    let mut first = first;
    let mut last = None;
    let mut pending = pending;
    loop {
        let depth = match pending.take() {
            Some(depth) => depth,
            None => match walk.descend(cursor) {
                Some(depth) => depth,
                None => break,
            },
        };
        let start = cursor.position();
        let code = cursor.read_int().expect("canonical skyline bits");
        let (negative, magnitude) = if first {
            first = false;
            (false, code)
        } else {
            unzigzag(code)
        };
        fold_signed_int(net, negative, &magnitude);
        extremum.fold(negative, &magnitude);
        last = Some((depth, cursor.position() - start));
    }
    last
}

/// Skip-scan the whole subtree at the cursor into a [`RegionSkip`]:
/// [`skip_leaves`] over a fresh walk, with the net movement and the streaming
/// minimum materialized.
///
/// # Panics
///
/// Panics if the stream is not a canonical skyline encoding.
pub(super) fn skip_region(cursor: &mut DsiCursor<'_>, first: bool) -> RegionSkip {
    let mut walk = LeafWalk::new();
    skip_leaves(&mut walk, cursor, first, None).expect("a subtree has at least one leaf")
}

/// Skip-scan the remaining leaves of a subtree whose walk is already open, or
/// `None` when none remain (`pending` as in [`fold_region`]).
///
/// The minimum is over the folded leaves alone: the first of them arms the
/// fold, so its height — not the caller's last consumed one — is the range's
/// entry extremum.
///
/// # Panics
///
/// Panics if the stream is not a canonical skyline encoding.
pub(super) fn skip_leaves(
    walk: &mut LeafWalk,
    cursor: &mut DsiCursor<'_>,
    first: bool,
    pending: Option<usize>,
) -> Option<RegionSkip> {
    let mut net = Accumulator::new();
    let mut min = Extremum::min(Accumulator::new());
    let (last_depth, last_code_len) =
        fold_region(walk, cursor, first, &mut net, &mut min, pending)?;
    let (net_sign, net_magnitude) = net.sign_magnitude();
    let (min_sign, min_magnitude) = min.into_offset().sign_magnitude();
    debug_assert_ne!(
        min_sign,
        Ordering::Greater,
        "the minimum is at or below the exit height"
    );
    Some(RegionSkip {
        net: Signed::from_sign_magnitude(net_sign, net_magnitude),
        min_from_exit: Signed::from_sign_magnitude(min_sign, min_magnitude),
        last_depth,
        last_code_len,
    })
}
