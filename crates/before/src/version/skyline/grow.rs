//! The grow splice on skyline streams: rebuild exactly one root-to-leaf path
//! along a recorded route, registering `k >= 1` events at once.
//!
//! `grow` registers new events when `fill` cannot simplify the tree, by
//! inflating the cheapest available leaf — cheapest by the lexicographic cost
//! `(expansions, depth)`, ties favoring the right child. The cost fold that
//! picks the path is not a pass of its own: the fused tick walk (the
//! [`fill`](super::fill) module) folds it in post-order over the same `(id,
//! event)` shape it is already traversing, recording which child the cheapest
//! inflation descends into at every id branch node, into the position-keyed
//! `Route`. When the walk's changed flag stays clear (`fill(i, e) = e`), `emit`
//! replays that route in one `O(n + m)` pass, compounding the whole `+k`
//! increment at the chosen leaf:
//!
//! - every off-path subtree is copied as a verbatim bit range;
//! - the inflation point is re-coded: the grown leaf's own delta and
//!   its preorder successor's are the only payload codes the height
//!   change can reach (`+k` and `−k`), and an expansion chain's fresh
//!   sibling leaves are `0`/`±k` deltas by construction;
//! - the output runs through the collapsing builder (the `build`
//!   sibling module), which performs any normalization the splice
//!   leaves reachable; off-path ranges pass through it untouched
//!   because the input was canonical.
//!
//! One `+k` splice equals `k` sequential single-event grows, byte for byte:
//! grow's cost is a function of the `(id, event)` topology alone — the fold
//! never reads a leaf value — and a free increment changes no topology (a
//! collapse would need the grown leaf to rise into equality with a sibling leaf
//! it was strictly below, which fill-fixedness forbids), so `k` sequential
//! grows re-derive the identical route and compound `+k` at one leaf. After an
//! expansion, the chain's terminal is the unique zero-expansion site — the
//! chain was chosen cheapest, so no zero-expansion site existed before it — and
//! the remaining `k − 1` events free-increment it: the terminal fresh leaf
//! simply carries `k`. The `ticks` differentials ([`fill`](super::fill)'s test
//! suite) hold this to the iterated public tick byte for byte.
//!
//! # The route's shape
//!
//! `emit` runs only on the unchanged branch, and on an unchanged tree a
//! fully-owned id region never covers an event node (`fill(1, e) = max(e)`
//! would have collapsed it), so every branch the chosen path can cross is an
//! *id* node — over an event node (descend both) or over an event leaf (an
//! expansion chain, walked id-only). One direction bit per id branch position
//! is therefore the whole channel, and the full-id-over-event-node arm is
//! asserted unreachable rather than routed.
//!
//! # The splice's boundary repairs
//!
//! Every consecutive-leaf delta between two leaves that both lie outside the
//! chosen path is unchanged by the inflation, so the emit copies off-path
//! subtrees as verbatim bit ranges and repairs exactly one boundary delta per
//! splice edge that can change:
//!
//! - the grown leaf's own code moves by `+k` (decoded, stepped,
//!   re-coded — the one payload the walk's cost fold never read);
//! - the first leaf *after* the inflation point moves by `−k` exactly
//!   when the grown leaf is its preorder predecessor;
//! - an expansion chain's fresh sibling leaves are `0`/`±k` deltas by
//!   construction, coded directly.
//!
//! The pre-chosen prefix — path node flags and whole left off-path subtrees —
//! precedes every changed height, so it is spliced with no repair at all.
//!
//! # Cost
//!
//! Derived, with the constants pinned by the tick rows of the resource-envelope
//! suite (`tests/meter.rs`) and the board's tick cells: the emit's splices copy
//! each off-path bit once and its repairs decode/re-code two payload codes;
//! transient state is the `Route` (one bit per id bit, recorded by the walk),
//! the pending- sibling path bits, and the builder's per-level stacks — no node
//! array, no per-level machine word, zero grown segments.
//!
//! # Testing
//!
//! The recursive oracle's `grow` is the behavioral witness on every pair the
//! splice is reachable for (the walk's changed flag clear): the ticked stream
//! must equal the oracle's normalized, encoded inflation byte for byte, over
//! the grow-branch members of the adversarial families crossed with adversarial
//! parties, arbitrary pairs, organic histories, and the exhaustive small scope;
//! the same scope is held to the brute-force minimal-inflation search directly,
//! not merely to another implementation of the same dynamic program. The
//! `Route` contract is pinned separately: the fused walk's bit vector must
//! equal a reference recursive probe's on every reachable pair — the explicit
//! guard against a walk/emit coordinate drift, which would misread a direction
//! silently rather than panic. Deep spines swap the native-frame oracle for
//! closed-form expected values.

use crate::codec::{self, Base, BitCursor, BitsMut, BitsSlice, Code};

use super::build::SkylineBuilder;
use super::signed::{gamma_code, gamma_code_signed, unzigzag_base, zigzag_signed};
use super::walk::LeafWalk;

/// Lexicographic inflation cost: prefer fewer leaf-to-node expansions, then a
/// shallower spot. The derived ordering is exactly that comparison — the field
/// order spells it, `expansions` before `depth`.
///
/// [`Cost::MAX`] marks an infeasible (empty-id) region. Ties between a node's
/// two children favor the *right* child (strict `<` picks the left only when
/// strictly cheaper). The fold itself rides the fused tick walk (`fill::fuse`);
/// this module owns the vocabulary because the [`Route`] the fold records is
/// the emit's input.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Cost {
    /// Leaf-to-node expansions along the inflation path.
    pub(super) expansions: u32,
    /// The inflation site's depth below the walked root.
    pub(super) depth: u32,
}

impl Cost {
    /// The cost of an infeasible region: an empty-id subtree can never be
    /// inflated.
    pub(super) const MAX: Cost = Cost {
        expansions: u32::MAX,
        depth: u32::MAX,
    };

    /// The zero inflation cost: a fully-owned terminal is a free increment
    /// (`grow(1, n) = (n + 1, 0)`).
    pub(super) const FREE: Cost = Cost {
        expansions: 0,
        depth: 0,
    };
}

/// The walk → emit channel: the cheapest inflation's route, one direction *bit*
/// per id branch node — `true` = descend the left child.
///
/// Keyed by branch position rather than stored as one linear path because the
/// emit walks only the chosen path while the recording walk visited every
/// branch, so the emit must look its direction up by where it is. Every branch
/// the emit can cross is an id node (module doc), keyed by its 2-bit tag's
/// position; each key is unique (each id node is reached once). One allocation,
/// `O(m)` bits, `O(1)` access, write order irrelevant. A bit defaults to
/// `false` (right); a walk/emit mismatch would misread a direction rather than
/// panic, which is why the route differential pins the walk's route against a
/// reference recursive probe bit for bit.
pub(super) struct Route {
    dirs: BitsMut,
}

impl Route {
    /// All directions cleared, sized to the id's bit positions.
    pub(super) fn new(id_span: usize) -> Self {
        Route {
            dirs: BitsMut::repeat(false, id_span),
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
/// Reads node flags and *skips* leaf payload codes by width — the emit never
/// materializes a height it does not re-code — and skips whole subtrees with a
/// pending-children counter. The reference recursive probe (`tests`) shares it.
/// The reads ride the word-parallel cursor; the node reads stay single-flag
/// because the walk interleaves with the id stream one node at a time — there
/// is no run to batch.
struct EvScan<'a> {
    bits: &'a BitsSlice,
    cursor: codec::DsiCursor<'a>,
}

impl<'a> EvScan<'a> {
    /// A cursor at the stream's root.
    fn new(bits: &'a BitsSlice) -> Self {
        EvScan {
            bits,
            cursor: codec::DsiCursor::new(bits),
        }
    }

    /// The cursor's bit position: the next node's flag.
    fn pos(&self) -> usize {
        self.cursor.position()
    }

    /// Move the cursor to `pos` (a node-flag position located by a side scan):
    /// `O(1)`, nothing is read or recorded.
    fn seek(&mut self, pos: usize) {
        self.cursor = codec::DsiCursor::new_at(self.bits, pos);
    }

    /// Decode the node at the cursor and advance past its header: `None` for an
    /// internal node (now at its left child), or the leaf's payload code range
    /// (now past it).
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    fn read(&mut self) -> Option<core::ops::Range<usize>> {
        let leaf = self.cursor.read_bit().expect("canonical skyline bits");
        if !leaf {
            None
        } else {
            let start = self.cursor.position();
            self.cursor.skip_int().expect("canonical skyline bits");
            Some(start..self.cursor.position())
        }
    }

    /// Advance past the whole subtree at the cursor without decoding anything.
    /// The reference recursive probe's infeasible arm (`tests`) is the one
    /// consumer: the emit itself never crosses an absent id child.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    #[cfg(test)]
    fn skip(&mut self) {
        // One unary read per descent: `k` internal nodes open two children
        // each, and the terminating leaf closes one.
        let mut pending = 1usize;
        while pending > 0 {
            let k = self.cursor.read_unary().expect("canonical skyline bits");
            self.cursor.skip_int().expect("canonical skyline bits");
            pending = pending + k - 1;
        }
    }
}

/// Decode the 2-bit id tag at `pos`: `(left_present, right_present)`.
///
/// Neither present is the full `1` terminal; a canonical id has no `(0, 0)`
/// node. `O(1)` random access into the packed id.
fn id_tag(bits: &BitsSlice, pos: usize) -> (bool, bool) {
    codec::scan::record_bits(2);
    (bits[pos], bits[pos + 1])
}

/// Position just past the id subtree whose tag sits at `pos`.
fn id_skip(bits: &BitsSlice, pos: usize) -> usize {
    crate::idbits::skip_subtree(pos, |at| {
        codec::scan::record_bits(2);
        let children = usize::from(bits[at]) + usize::from(bits[at + 1]);
        (children, at + 2)
    })
}

/// The boundary repair a spliced subtree's first payload code needs.
#[derive(Clone, Copy)]
enum Repair<'k> {
    /// The predecessor leaf is unchanged: copy the code verbatim.
    None,
    /// The predecessor is the grown leaf, `k` higher than it was: the delta
    /// drops by `k`.
    Minus(&'k Base),
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

/// Locate the subtree at `start`: its end, and the first/last leaf coordinates
/// the verbatim splice re-anchors the builder around.
///
/// One forward pass over the subtree's bits with a relative-path bit stack —
/// the same walk the leaf cursors do, restricted to one subtree.
///
/// # Panics
///
/// Panics if the stream is not a canonical skyline encoding.
fn scan_subtree(bits: &BitsSlice, start: usize) -> Subtree {
    let mut cursor = codec::DsiCursor::new_at(bits, start);
    // The first leaf's coordinates, recorded once; the last leaf's are whatever
    // the loop recorded most recently when the walk ends.
    let mut first: Option<(core::ops::Range<usize>, usize)> = None;
    let mut last_code = 0..0;
    let mut last_rel_depth = 0;
    let mut walk = LeafWalk::new();
    while let Some(depth) = walk.descend(&mut cursor) {
        let code_start = cursor.position();
        cursor.skip_int().expect("canonical skyline bits");
        last_code = code_start..cursor.position();
        last_rel_depth = depth;
        if first.is_none() {
            first = Some((last_code.clone(), last_rel_depth));
        }
    }
    let (first_code, first_rel_depth) = first.expect("a subtree has at least one leaf");
    Subtree {
        end: cursor.position(),
        first_code,
        first_rel_depth,
        last_code,
        last_rel_depth,
    }
}

/// Feed one whole off-path subtree from the cursor into the builder, rooted at
/// `depth`.
///
/// The first leaf goes through the builder's collapse checks (with the
/// successor repair when the grown leaf precedes it); the remainder is one
/// verbatim splice.
fn feed_subtree(out: &mut SkylineBuilder, ev: &mut EvScan<'_>, depth: usize, repair: Repair<'_>) {
    let info = scan_subtree(ev.bits, ev.pos());
    let orig = &ev.bits[info.first_code.clone()];
    let first_code = match repair {
        Repair::None => Code::from_slice(orig),
        // The successor is never the stream's first leaf (the grown leaf
        // precedes it), so its code is always a zigzag delta.
        Repair::Minus(k) => recode(orig, Step::DownDelta, k),
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
    ev.seek(info.end);
}

/// The three height steppings [`recode`] performs.
///
/// A stream's first leaf carries an absolute height and every later leaf a
/// zigzag delta; only the grown leaf itself ever steps *up*. A down-stepped
/// absolute height is therefore not a stepping at all — the variant does not
/// exist, rather than being asserted away.
#[derive(Clone, Copy)]
enum Step {
    /// The grown leaf as the stream's first: its absolute height rises
    /// by `k`.
    UpAbsolute,
    /// The grown leaf behind a predecessor: its zigzag delta rises by
    /// `k`.
    UpDelta,
    /// The grown leaf's successor: its zigzag delta drops by `k`, undoing the
    /// raise the grown leaf's own step introduced.
    DownDelta,
}

/// Re-code one leaf payload with its height stepped by `k`, per [`Step`].
///
/// One decode, one signed step, one re-encode — `O(the code's own width + the
/// width of k)`, the only payload arithmetic in the whole emit.
fn recode(code: &BitsSlice, step: Step, k: &Base) -> Code {
    let (value, end) = codec::decode_int(code, 0).expect("canonical skyline bits");
    debug_assert_eq!(end, code.len(), "a payload range is exactly one code");
    let increment = match step {
        Step::UpAbsolute => return gamma_code(&(value + k)),
        Step::UpDelta => true,
        Step::DownDelta => false,
    };
    let (negative, magnitude) = unzigzag_base(value);
    let stepped = match (increment, negative) {
        // Stepping a nonnegative delta up, or a negative one further down,
        // grows the magnitude.
        (true, false) | (false, true) => zigzag_signed(negative, magnitude + k),
        // Stepping a nonnegative delta down past zero: the sign flips and the
        // magnitude is the overshoot — `k` itself from a zero magnitude, read
        // off the unmetered width so the zero case costs exactly its
        // comparison.
        (false, false) if magnitude < *k => {
            let over = if magnitude.bits() == 0 {
                k.clone()
            } else {
                k.clone() - &magnitude
            };
            zigzag_signed(true, over)
        }
        // Otherwise the magnitude shrinks by `k`; a shrink to exactly zero
        // lands on the positive zero. The increment-on-negative arm can cross
        // zero only at `k > magnitude >= 1`, which the width tests decide for
        // free at `k = 1` (one bit wide) and route through one comparison only
        // when the widths tie.
        (true, true) | (false, false) => {
            let crosses = negative
                && (magnitude.bits() < k.bits()
                    || (magnitude.bits() == k.bits() && k.bits() > 1 && magnitude < *k));
            if crosses {
                zigzag_signed(false, k.clone() - &magnitude)
            } else {
                let shrunk = magnitude - k;
                zigzag_signed(negative && shrunk != Base::ZERO, shrunk)
            }
        }
    };
    gamma_code(&stepped)
}

/// Emit the grown stream: replay `route` along the chosen path and register `k`
/// events at the inflation site in one `+k` compound.
///
/// Everything off the path is spliced verbatim, the boundary deltas repair by
/// `±k`, and the output runs through the builder, which collapses anything the
/// splice leaves collapsible; the module doc carries why one compound equals
/// `k` sequential grows.
///
/// `route` is the fused tick walk's record over exactly this `(ev, id)` pair,
/// and the pair is one the walk left unchanged — the caller
/// (`fill::tick`/`fill::ticks`) established both. The id must own at least one
/// region and `k` must be at least 1; the result otherwise is unspecified in
/// release builds (debug builds panic).
pub(super) fn emit(ev_bits: &BitsSlice, id_bits: &BitsSlice, route: &Route, k: &Base) -> BitsMut {
    debug_assert!(
        !id_bits.is_empty(),
        "grow requires an id owning at least one region"
    );
    // The width test keeps the guard off the limb meter: a dev-profile meter
    // reading must match the release reading on this path.
    debug_assert!(k.bits() != 0, "the splice registers at least one event");
    let mut ev = EvScan::new(ev_bits);
    let mut id_pos = 0usize;
    // Subadditivity of the coding bounds the output by the input plus the
    // expansion chain's fresh codes, each a few bits per id level.
    let mut out = SkylineBuilder::with_capacity(ev_bits.len() + id_bits.len() + 64);
    // One bit per chosen-path level: `true` = the branch descended left, so its
    // right sibling subtree is pending after the inflation point.
    let mut pending = BitsMut::new();
    let mut depth = 0usize;
    // Whether any leaf has entered the output ahead of the grown leaf. The
    // grown leaf's own code is absolute exactly when none has (Phase 2's
    // UpAbsolute/UpDelta selection): the decision is the walk's, not recode's,
    // because only the walk knows what it fed.
    let mut fed_any = false;

    // Phase 1: descend the chosen path, splicing left off-path subtrees as they
    // pass, until the inflation point: an event leaf under a full id (increment
    // in place) or under an id node (expansion chain, walked id-only). A `loop
    // { .. break (..) }` block rather than a helper function: the descent
    // mutates the walk's whole local state (the event scan, the id position,
    // the builder, the pending-sibling bits, the path depth, `fed_any`), all of
    // which the later phases keep using, so a function boundary would thread
    // every one of them.
    let (orig_code, chain_dirs) = loop {
        let key = id_pos;
        let (l, r) = id_tag(id_bits, id_pos);
        id_pos += 2;
        if !l && !r {
            // A fully-owned terminal: on the unchanged branch it covers a
            // single leaf (over an event node, `fill(1, e) = max(e)` would have
            // collapsed the region and tripped the changed flag), and
            // incrementing that leaf in place is the inflation.
            match ev.read() {
                Some(code) => break (code, BitsMut::new()),
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
            // An id node over an event leaf — the chain below is id-only, its
            // directions collected for the fresh leaves' preorder.
            Some(code) => {
                let mut dirs = BitsMut::new();
                let mut cur = (key, l, r);
                loop {
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

    // Phase 2: the inflation point, with the grown height `h + k`. The original
    // leaf spanned depth `d0`; a chain of expansions roots a fresh subtree
    // there whose leaves are the original height `h` everywhere except the
    // grown leaf's `h + k` at the bottom — so every fresh code is a `0`/`±k`
    // delta, except the chain's first leaf, which keeps the original code (same
    // height, same predecessor) or re-codes it `+k` when the grown leaf itself
    // comes first.
    let d0 = depth;
    let chain = chain_dirs.len();
    let orig = &ev_bits[orig_code];
    debug_assert_eq!(d0, pending.len(), "one pending record per path level");
    let mut emitted_in_chain = false;
    // Fresh sibling leaves that precede the grown leaf: one per level whose
    // branch descended right (the sibling is the left child).
    for j in 0..chain {
        if !chain_dirs[j] {
            let code = if emitted_in_chain {
                gamma_code(&Base::ZERO)
            } else {
                Code::from_slice(orig)
            };
            out.leaf(d0 + j + 1, code);
            emitted_in_chain = true;
        }
    }
    let grown_code = if emitted_in_chain {
        gamma_code_signed(false, k)
    } else {
        // With nothing fed before it, the grown leaf is the output's first: its
        // code is the absolute height, not a delta.
        let step = if fed_any {
            Step::UpDelta
        } else {
            Step::UpAbsolute
        };
        recode(orig, step, k)
    };
    out.leaf(d0 + chain, grown_code);
    // Fresh sibling leaves that follow the grown leaf, deepest first.
    let mut first_after_grown = true;
    for j in (0..chain).rev() {
        if chain_dirs[j] {
            let code = if first_after_grown {
                gamma_code_signed(true, k)
            } else {
                gamma_code(&Base::ZERO)
            };
            out.leaf(d0 + j + 1, code);
            first_after_grown = false;
        }
    }

    // Phase 3: unwind. The pending right subtrees follow the inflation point
    // contiguously in the input, deepest first; only the first one's first leaf
    // can need the `−k` repair, and only when the grown leaf (not a trailing
    // fresh sibling) is the last leaf the inflation emitted.
    let mut repair = if first_after_grown {
        Repair::Minus(k)
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
