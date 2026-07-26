//! The grow splice on skyline streams: rebuild exactly one root-to-leaf
//! path along a recorded route.
//!
//! `grow` registers a new event when `fill` cannot simplify the tree, by
//! inflating the cheapest available leaf — cheapest by the lexicographic
//! cost `(expansions, depth)`, ties favoring the right child. The cost
//! fold that picks the path is not a pass of its own: the fused tick
//! walk (the [`fill`](super::fill) module) folds it in post-order over
//! the same `(id, event)` shape it is already traversing, recording
//! which child the cheapest inflation descends into at every id branch
//! node, into the position-keyed `Route`. When the walk's changed
//! flag stays clear (`fill(i, e) = e`), `emit` replays that route in
//! one `O(n + m)` pass:
//!
//! - every off-path subtree is copied as a verbatim bit range;
//! - the inflation point is re-coded: the grown leaf's own delta and
//!   its preorder successor's are the only payload codes the height
//!   change can reach, and an expansion chain's fresh sibling leaves
//!   are `0`/`±1` deltas by construction;
//! - the output runs through the collapsing builder (the `build`
//!   sibling module), which performs any normalization the splice
//!   leaves reachable; off-path ranges pass through it untouched
//!   because the input was canonical.
//!
//! # The route's shape
//!
//! `emit` runs only on the unchanged branch, and on an unchanged tree a
//! fully-owned id region never covers an event node (`fill(1, e) =
//! max(e)` would have collapsed it), so every branch the chosen path
//! can cross is an *id* node — over an event node (descend both) or
//! over an event leaf (an expansion chain, walked id-only). One
//! direction bit per id branch position is therefore the whole channel,
//! and the full-id-over-event-node arm is asserted unreachable rather
//! than routed.
//!
//! # The splice's boundary repairs
//!
//! Every consecutive-leaf delta between two leaves that both lie outside
//! the chosen path is unchanged by the inflation, so the emit copies
//! off-path subtrees as verbatim bit ranges and repairs exactly one
//! boundary delta per splice edge that can change:
//!
//! - the grown leaf's own code moves by `+1` (decoded, stepped,
//!   re-coded — the one payload the walk's cost fold never read);
//! - the first leaf *after* the inflation point moves by `−1` exactly
//!   when the grown leaf is its preorder predecessor;
//! - an expansion chain's fresh sibling leaves are `0`/`±1` deltas by
//!   construction, coded directly.
//!
//! The pre-chosen prefix — path node flags and whole left off-path
//! subtrees — precedes every changed height, so it is spliced with no
//! repair at all.
//!
//! # Cost
//!
//! Derived, with the constants pinned by the tick rows of the
//! resource-envelope suite (`tests/meter.rs`) and the board's tick
//! cells: the emit's splices copy each off-path bit once and its
//! repairs decode/re-code two payload codes; transient state is the
//! `Route` (one bit per id bit, recorded by the walk), the pending-
//! sibling path bits, and the builder's per-level stacks — no node
//! array, no per-level machine word, zero grown segments.
//!
//! # Testing
//!
//! The recursive oracle's `grow` is the behavioral witness on every
//! pair the splice is reachable for (the walk's changed flag clear):
//! the ticked stream must equal the oracle's normalized, encoded
//! inflation byte for byte, over the grow-branch members of the
//! adversarial families crossed with adversarial parties, arbitrary
//! pairs, organic histories, and the exhaustive small scope; the same
//! scope is held to the brute-force minimal-inflation search directly,
//! not merely to another implementation of the same dynamic program.
//! The `Route` contract is pinned separately: the fused walk's bit
//! vector must equal a reference recursive probe's on every reachable
//! pair — the explicit guard against a walk/emit coordinate drift,
//! which would misread a direction silently rather than panic. Deep
//! spines swap the native-frame oracle for closed-form expected values.

use crate::codec::{self, Base, Bits, BitsSlice};
use crate::step;

use super::build::SkylineBuilder;
use super::{gamma_code, unzigzag, zigzag_signed};

/// Lexicographic inflation cost `(expansions, depth)`: prefer fewer
/// leaf-to-node expansions, then a shallower spot.
///
/// `MAX` ([`COST_MAX`]) marks an infeasible (empty-id) region. Ties
/// between a node's two children favor the *right* child (strict `<`
/// picks the left only when strictly cheaper). The fold itself rides
/// the fused tick walk (`fill::fuse`); this module owns the vocabulary
/// because the [`Route`] the fold records is the emit's input.
pub(super) type Cost = (u32, u32);

/// The cost of an infeasible region: an empty-id subtree can never be
/// inflated.
pub(super) const COST_MAX: Cost = (u32::MAX, u32::MAX);

/// The walk → emit channel: the cheapest inflation's route, one
/// direction *bit* per id branch node — `true` = descend the left
/// child.
///
/// Keyed by branch position rather than stored as one linear path
/// because the emit walks only the chosen path while the recording walk
/// visited every branch, so the emit must look its direction up by
/// where it is. Every branch the emit can cross is an id node (module
/// doc), keyed by its 2-bit tag's position; each key is unique (each id
/// node is reached once). One allocation, `O(m)` bits, `O(1)` access,
/// write order irrelevant. A bit defaults to `false` (right); a
/// walk/emit mismatch would misread a direction rather than panic,
/// which is why the route differential pins the walk's route against a
/// reference recursive probe bit for bit.
pub(super) struct Route {
    dirs: Bits,
}

impl Route {
    /// All directions cleared, sized to the id's bit positions.
    pub(super) fn new(id_span: usize) -> Self {
        Route {
            dirs: Bits::repeat(false, id_span),
        }
    }

    /// Record that the cheapest inflation at the branch keyed by `key`
    /// descends into the left child (`left = true`).
    pub(super) fn record(&mut self, key: usize, left: bool) {
        self.dirs.set(key, left);
    }

    /// Whether the cheapest inflation at the branch keyed by `key`
    /// descends into the left child.
    fn descends_left(&self, key: usize) -> bool {
        self.dirs[key]
    }

    /// The raw direction bits, for the route differential.
    #[cfg(test)]
    pub(super) fn dirs(&self) -> &BitsSlice {
        &self.dirs
    }
}

/// A forward topology cursor over one skyline stream.
///
/// Reads node flags and *skips* leaf payload codes by width — the emit
/// never materializes a height it does not re-code — and skips whole
/// subtrees with a pending-children counter. The reference recursive
/// probe (`tests`) shares it.
struct EvScan<'a> {
    bits: &'a BitsSlice,
    pos: usize,
}

impl<'a> EvScan<'a> {
    /// A cursor at the stream's root.
    fn new(bits: &'a BitsSlice) -> Self {
        EvScan { bits, pos: 0 }
    }

    /// The cursor's bit position: the next node's flag.
    fn pos(&self) -> usize {
        self.pos
    }

    /// Decode the node at the cursor and advance past its header:
    /// `None` for an internal node (now at its left child), or the
    /// leaf's payload code range (now past it).
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    fn read(&mut self) -> Option<core::ops::Range<usize>> {
        step!();
        codec::scan::record_bits(1);
        let internal = self.bits[self.pos];
        self.pos += 1;
        if internal {
            None
        } else {
            let start = self.pos;
            self.pos = codec::skip_int(self.bits, start).expect("canonical skyline bits");
            Some(start..self.pos)
        }
    }

    /// Advance past the whole subtree at the cursor without decoding
    /// anything. The reference recursive probe's infeasible arm
    /// (`tests`) is the one consumer: the emit itself never crosses an
    /// absent id child.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    #[cfg(test)]
    fn skip(&mut self) {
        let bits = self.bits;
        self.pos = crate::idbits::skip_subtree(self.pos, |at| {
            step!();
            codec::scan::record_bits(1);
            if bits[at] {
                (2, at + 1)
            } else {
                let next = codec::skip_int(bits, at + 1).expect("canonical skyline bits");
                (0, next)
            }
        });
    }
}

/// Decode the 2-bit id tag at `pos`: `(left_present, right_present)`.
///
/// Neither present is the full `1` terminal; a canonical id has no
/// `(0, 0)` node. `O(1)` random access into the packed id.
fn id_tag(bits: &BitsSlice, pos: usize) -> (bool, bool) {
    step!();
    codec::scan::record_bits(2);
    (bits[pos], bits[pos + 1])
}

/// Position just past the id subtree whose tag sits at `pos`.
fn id_skip(bits: &BitsSlice, pos: usize) -> usize {
    crate::idbits::skip_subtree(pos, |at| {
        step!();
        codec::scan::record_bits(2);
        let children = usize::from(bits[at]) + usize::from(bits[at + 1]);
        (children, at + 2)
    })
}

/// The boundary repair a spliced subtree's first payload code needs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Repair {
    /// The predecessor leaf is unchanged: copy the code verbatim.
    None,
    /// The predecessor is the grown leaf, one higher than it was: the
    /// delta drops by one.
    MinusOne,
}

/// One complete off-path subtree, located by a forward topology scan.
struct Subtree {
    /// Just past the subtree's last bit.
    end: usize,
    /// The first (leftmost) leaf's payload code range.
    first_code: core::ops::Range<usize>,
    /// The first leaf's depth below the subtree root; `0` means the
    /// subtree is a single leaf.
    first_rel_depth: usize,
    /// The last (rightmost) leaf's payload code range.
    last_code: core::ops::Range<usize>,
    /// The last leaf's depth below the subtree root.
    last_rel_depth: usize,
}

/// Locate the subtree at `start`: its end, and the first/last leaf
/// coordinates the verbatim splice re-anchors the builder around.
///
/// One forward pass over the subtree's bits with a relative-path bit
/// stack — the same walk the leaf cursors do, restricted to one subtree.
///
/// # Panics
///
/// Panics if the stream is not a canonical skyline encoding.
fn scan_subtree(bits: &BitsSlice, start: usize) -> Subtree {
    let mut pos = start;
    let mut path = Bits::new();
    // Descend to the leftmost leaf.
    loop {
        step!();
        codec::scan::record_bits(1);
        let internal = bits[pos];
        pos += 1;
        if !internal {
            break;
        }
        path.push(false);
    }
    let code_start = pos;
    pos = codec::skip_int(bits, pos).expect("canonical skyline bits");
    let first_code = code_start..pos;
    let first_rel_depth = path.len();
    let mut last_code = first_code.clone();
    let mut last_rel_depth = first_rel_depth;
    loop {
        // Close the ancestors the consumed leaf completed; an emptied
        // path means it was the subtree's last leaf.
        let mut flipped = false;
        while let Some(bit) = path.pop() {
            if !bit {
                path.push(true);
                flipped = true;
                break;
            }
        }
        if !flipped {
            break;
        }
        // Descend to the next leaf.
        loop {
            step!();
            codec::scan::record_bits(1);
            let internal = bits[pos];
            pos += 1;
            if !internal {
                break;
            }
            path.push(false);
        }
        let code_start = pos;
        pos = codec::skip_int(bits, pos).expect("canonical skyline bits");
        last_code = code_start..pos;
        last_rel_depth = path.len();
    }
    Subtree {
        end: pos,
        first_code,
        first_rel_depth,
        last_code,
        last_rel_depth,
    }
}

/// Feed one whole off-path subtree from the cursor into the builder,
/// rooted at `depth`.
///
/// The first leaf goes through the builder's collapse checks (with the
/// successor repair when the grown leaf precedes it); the remainder is
/// one verbatim splice.
fn feed_subtree(out: &mut SkylineBuilder, ev: &mut EvScan<'_>, depth: usize, repair: Repair) {
    let info = scan_subtree(ev.bits, ev.pos);
    let orig = &ev.bits[info.first_code.clone()];
    let first_code = match repair {
        Repair::None => orig.to_bitvec(),
        // The successor is never the stream's first leaf (the grown
        // leaf precedes it), so its code is always a zigzag delta.
        Repair::MinusOne => recode(orig, false, false),
    };
    out.leaf(depth + info.first_rel_depth, first_code);
    if info.first_rel_depth > 0 {
        out.continue_verbatim(
            &ev.bits[info.first_code.end..info.end],
            depth,
            info.last_rel_depth,
            info.last_code.len(),
        );
    }
    ev.pos = info.end;
}

/// Re-code one leaf payload with its height stepped by one:
/// `increment` up for the grown leaf, down for its successor.
///
/// `absolute` distinguishes the stream's first leaf (a plain gamma
/// height) from every later one (a zigzag delta). One decode, one
/// signed step, one re-encode — `O(the code's own width)`, the only
/// payload arithmetic in the whole emit.
fn recode(code: &BitsSlice, increment: bool, absolute: bool) -> Bits {
    let (value, end) = codec::decode_int(code, 0).expect("canonical skyline bits");
    debug_assert_eq!(end, code.len(), "a payload range is exactly one code");
    let one = Base::from(1u8);
    if absolute {
        debug_assert!(increment, "only the grown leaf re-codes an absolute height");
        return gamma_code(&(value + 1u32));
    }
    let (negative, magnitude) = unzigzag(value);
    let stepped = match (increment, negative) {
        // Stepping a nonnegative delta up, or a negative one further
        // down, grows the magnitude.
        (true, false) | (false, true) => zigzag_signed(negative, magnitude + 1u32),
        // Stepping across zero: `0 − 1` is the one sign flip (a
        // negative delta's magnitude is at least 1, so `−m + 1` never
        // crosses).
        (false, false) if magnitude == Base::ZERO => zigzag_signed(true, one),
        // Otherwise the magnitude shrinks toward zero; a magnitude of
        // exactly 1 lands on the positive zero.
        (true, true) | (false, false) => {
            let shrunk = magnitude - &one;
            zigzag_signed(negative && shrunk != Base::ZERO, shrunk)
        }
    };
    gamma_code(&stepped)
}

/// Emit the grown stream: replay `route` along the chosen path, splice
/// everything off it, repair the boundary deltas, and let the builder
/// collapse anything the splice leaves collapsible.
///
/// `route` is the fused tick walk's record over exactly this `(ev, id)`
/// pair, and the pair is one the walk left unchanged — the caller
/// (`fill::tick`) established both. The id must own at least one
/// region; the result on an empty id is unspecified in release builds
/// (debug builds panic).
pub(super) fn emit(ev_bits: &BitsSlice, id_bits: &BitsSlice, route: &Route) -> Bits {
    debug_assert!(
        !id_bits.is_empty(),
        "grow requires an id owning at least one region"
    );
    let mut ev = EvScan::new(ev_bits);
    let mut id_pos = 0usize;
    // Subadditivity of the coding bounds the output by the input plus
    // the expansion chain's fresh codes, each a few bits per id level.
    let mut out = SkylineBuilder::with_capacity(ev_bits.len() + id_bits.len() + 64);
    // One bit per chosen-path level: `true` = the branch descended left,
    // so its right sibling subtree is pending after the inflation point.
    let mut pending = Bits::new();
    let mut depth = 0usize;
    let mut fed_any = false;

    // Phase 1: descend the chosen path, splicing left off-path subtrees
    // as they pass, until the inflation point: an event leaf under a
    // full id (increment in place) or under an id node (expansion
    // chain, walked id-only).
    let (orig_code, chain_dirs) = loop {
        step!();
        let key = id_pos;
        let (l, r) = id_tag(id_bits, id_pos);
        id_pos += 2;
        if !l && !r {
            // A fully-owned terminal: on the unchanged branch it covers
            // a single leaf (over an event node, `fill(1, e) = max(e)`
            // would have collapsed the region and tripped the changed
            // flag), and incrementing that leaf in place is the
            // inflation.
            match ev.read() {
                Some(code) => break (code, Bits::new()),
                None => unreachable!("a full id over an event node collapses under fill"),
            }
        }
        match ev.read() {
            // An id node over an event node: one more path level.
            None => {
                if route.descends_left(key) {
                    debug_assert!(l, "the chosen path never enters an absent id child");
                    pending.push(true);
                } else {
                    debug_assert!(r, "the chosen path never enters an absent id child");
                    feed_subtree(&mut out, &mut ev, depth + 1, Repair::None);
                    if l {
                        id_pos = id_skip(id_bits, id_pos);
                    }
                    fed_any = true;
                    pending.push(false);
                }
                depth += 1;
            }
            // An id node over an event leaf — the chain below is
            // id-only, its directions collected for the fresh leaves'
            // preorder.
            Some(code) => {
                let mut dirs = Bits::new();
                let mut cur = (key, l, r);
                loop {
                    step!();
                    let (key, l, r) = cur;
                    let left = route.descends_left(key);
                    if left {
                        debug_assert!(l, "the chosen path never enters an absent id child");
                    } else {
                        debug_assert!(r, "the chosen path never enters an absent id child");
                        if l {
                            id_pos = id_skip(id_bits, id_pos);
                        }
                    }
                    dirs.push(left);
                    let next = (id_pos, id_tag(id_bits, id_pos));
                    id_pos += 2;
                    let (nkey, (nl, nr)) = next;
                    if !nl && !nr {
                        break;
                    }
                    cur = (nkey, nl, nr);
                }
                break (code, dirs);
            }
        }
    };

    // Phase 2: the inflation point. The original leaf spanned depth
    // `d0`; a chain of `k` expansions roots a fresh subtree there whose
    // leaves are the original height `h` everywhere except the grown
    // leaf's `h + 1` at the bottom — so every fresh code is a `0`/`±1`
    // delta, except the chain's first leaf, which keeps the original
    // code (same height, same predecessor) or re-codes it `+1` when the
    // grown leaf itself comes first.
    let d0 = depth;
    let k = chain_dirs.len();
    let orig = &ev_bits[orig_code];
    debug_assert_eq!(d0, pending.len(), "one pending record per path level");
    let mut emitted_in_chain = false;
    // Fresh sibling leaves that precede the grown leaf: one per level
    // whose branch descended right (the sibling is the left child).
    for j in 0..k {
        if !chain_dirs[j] {
            let code = if emitted_in_chain {
                gamma_code(&Base::ZERO)
            } else {
                orig.to_bitvec()
            };
            out.leaf(d0 + j + 1, code);
            emitted_in_chain = true;
        }
    }
    let grown_code = if emitted_in_chain {
        gamma_code(&zigzag_signed(false, Base::from(1u8)))
    } else {
        recode(orig, true, !fed_any)
    };
    out.leaf(d0 + k, grown_code);
    // Fresh sibling leaves that follow the grown leaf, deepest first.
    let mut first_after_grown = true;
    for j in (0..k).rev() {
        if chain_dirs[j] {
            let code = if first_after_grown {
                gamma_code(&zigzag_signed(true, Base::from(1u8)))
            } else {
                gamma_code(&Base::ZERO)
            };
            out.leaf(d0 + j + 1, code);
            first_after_grown = false;
        }
    }

    // Phase 3: unwind. The pending right subtrees follow the inflation
    // point contiguously in the input, deepest first; only the first
    // one's first leaf can need the `−1` repair, and only when the
    // grown leaf (not a trailing fresh sibling) is the last leaf the
    // inflation emitted.
    let mut repair = if first_after_grown {
        Repair::MinusOne
    } else {
        Repair::None
    };
    for level in (0..d0).rev() {
        let went_left = pending.pop().expect("one pending record per path level");
        if went_left {
            feed_subtree(&mut out, &mut ev, level + 1, repair);
            repair = Repair::None;
        }
    }
    debug_assert_eq!(ev.pos(), ev_bits.len(), "the emit consumes the event");
    out.finish()
}

#[cfg(test)]
mod tests;
