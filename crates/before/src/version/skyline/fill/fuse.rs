//! The fused tick's walk-side state: the changed flag and the grow
//! route, riding the fill walk so `tick` is one pass plus one splice.
//!
//! The paper's `event` runs `fill` and keeps its output iff it moved
//! the tree, else registers the event by the cheapest inflation. Fused,
//! the one walk carries two extra pieces of state, each in its own
//! struct here:
//!
//! - [`Out`], the **changed flag as an output mode**. Until an emitted
//!   plateau differs from the input plateau it replaces, the output *is*
//!   the input prefix, so the walk runs as a verbatim reference — no
//!   builder, no materialization, just the matched prefix's end
//!   position. The first divergence materializes that prefix into a
//!   real [`SkylineBuilder`] (one wholesale copy, priced by the
//!   divergence that ends the run) and the walk continues exactly as a
//!   direct fill emission. A walk that never diverges *is* the flag
//!   reading false: `fill(i, e) = e`, byte-exact by canonical
//!   uniqueness, and fill's discarded output was never built at all.
//!   The flag's equivalence to that byte equality is pinned by the
//!   committed differentials of record (`fill/tests.rs`): the flag
//!   against the recursive oracle's `fill`, and the fused `tick`
//!   against the oracle's `event`, across every committed family, the
//!   exhaustive small scope, and proptest arbitraries.
//!
//! - [`RouteProbe`], **grow's cost fold as a passenger**. The
//!   inflation's dynamic program — lexicographic `(expansions, depth)`
//!   cost, ties to the right — visits exactly the `(id, event)` nodes
//!   the fill walk visits, so the walk folds it in post-order and
//!   records the chosen direction per id branch node into the
//!   [`Route`] the splice emit replays. The id subtrees the fill walk
//!   lazily skips under an event leaf (`fill((il, ir), n) = n`) are the
//!   one place the route needs reads the walk does not: there the fold
//!   rides the skip itself ([`RouteProbe::expand`] reads the same 2-bit
//!   tags `IdReader::skip` would). The probe dies at the first
//!   divergence — a tripped flag routes the pair to the fill branch,
//!   where no route is ever read — so the fill branch pays the route
//!   tax only over its matched prefix.
//!
//! The changed flag is a **first-divergence detector**, aligned by
//! plateau `(depth, code)`, never by "an arm fired": a raise that
//! reproduces the existing leaf value exactly must not trip it. While
//! every emitted plateau equals its input counterpart, output position
//! ≡ input position exactly — in particular the first emitted leaf
//! compares its absolute code against the input's absolute code, never
//! a delta against an absolute — and a collapse that shifts which leaf
//! is first trips on topology (the replaced range was not a single
//! leaf) before any code comparison is reached.

use crate::codec::{BitCursor, BitStack, BitsMut, BitsSlice, Code, PopStack};
use crate::idbits::{IdNode, IdReader};

use super::super::build::SkylineBuilder;
use super::super::grow::{Cost, Route, COST_MAX};
use super::super::walk::LeafWalk;

/// The zero inflation cost: a fully-owned terminal is a free increment
/// (`grow(1, n) = (n + 1, 0)`).
pub(super) const COST_FREE: Cost = (0, 0);

/// The [`PopStack`] encoding of one deferred route-DP quantity (a
/// [`Cost`] component, or an expansion chain's distance): 0 =
/// infeasible (a `u32::MAX` component), else the value + 1.
pub(super) fn encode_cost_component(v: u32) -> u64 {
    if v == u32::MAX {
        0
    } else {
        u64::from(v) + 1
    }
}

/// Invert [`encode_cost_component`].
pub(super) fn decode_cost_component(v: u64) -> u32 {
    if v == 0 {
        u32::MAX
    } else {
        (v - 1) as u32
    }
}

/// The fill walk's output, as the changed flag's realization
/// (module doc): a verbatim reference over the matched input prefix, or
/// a materialized builder after the first divergence.
// One `Out` lives per fill walk — never a collection element — so the
// variant-size gap prices nothing; boxing the builder would put a heap
// indirection on every emitted leaf instead.
#[allow(clippy::large_enum_variant)]
pub(super) enum Out {
    /// Every emitted plateau so far equals the input plateau it
    /// replaces, so the output so far is byte-identical to the input
    /// prefix ending at `matched_end` — nothing is built.
    Verbatim {
        /// The input position just past the last matched plateau's
        /// code: the prefix a divergence materializes.
        matched_end: usize,
    },
    /// A plateau diverged (or the walk replayed the prefix): the
    /// canonical builder holds the real output.
    Built(SkylineBuilder),
}

impl Out {
    /// A fresh verbatim reference at the stream's start.
    pub(super) fn verbatim() -> Self {
        Out::Verbatim { matched_end: 0 }
    }

    /// Whether the walk is still a verbatim reference (no divergence).
    pub(super) fn is_verbatim(&self) -> bool {
        matches!(self, Out::Verbatim { .. })
    }

    /// Record that the emission in flight matches its input plateau,
    /// whose code ends at `end`.
    ///
    /// Returns whether the caller may skip
    /// the emission body outright (a matched verbatim emission does no
    /// output work at all); on a built output this is a no-op
    /// answering false — emission bodies always run post-divergence.
    pub(super) fn note_match(&mut self, end: usize) -> bool {
        match self {
            Out::Verbatim { matched_end } => {
                *matched_end = end;
                true
            }
            Out::Built(_) => false,
        }
    }

    /// Append the next output plateau (the emission bodies' one sink).
    ///
    /// Unreachable in a verbatim walk: matched emissions return before
    /// their bodies, and diverging ones materialize first.
    pub(super) fn leaf(&mut self, depth: usize, code: Code) {
        match self {
            Out::Built(builder) => builder.leaf(depth, code),
            Out::Verbatim { .. } => {
                unreachable!("a verbatim emission is matched or has diverged")
            }
        }
    }

    /// Materialize the matched prefix at the first divergence.
    ///
    /// Decodes
    /// `ev[..matched_end]` — byte-identical to the output so far by the
    /// verbatim invariant — and feeds its plateaus through a fresh
    /// builder, leaving `self` built; a no-op once built.
    ///
    /// One wholesale prefix copy, priced by the divergence that ends
    /// the verbatim run; the walk from here on is a direct fill
    /// emission. Iterative (a path bit stack, no recursion), so prefix
    /// depth cannot overflow the native stack.
    pub(super) fn materialize(&mut self, ev: &BitsSlice) {
        let Out::Verbatim { matched_end, .. } = self else {
            return;
        };
        let matched_end = *matched_end;
        let mut builder = SkylineBuilder::with_capacity(ev.len());
        let mut cursor = crate::codec::DsiCursor::new(ev);
        let mut walk = LeafWalk::new();
        while cursor.position() < matched_end {
            // A descent never straddles `matched_end` (a matched prefix
            // ends on a plateau boundary), and a divergence always
            // leaves its own consumed range beyond the matched prefix,
            // so the prefix is a proper prefix of the tiling and some
            // ancestor is still open at its end — the walk cannot
            // exhaust before the position bound stops this loop.
            let depth = walk
                .descend(&mut cursor)
                .expect("a matched prefix is a proper prefix of the tiling");
            let start = cursor.position();
            cursor.skip_int().expect("canonical skyline bits");
            builder.leaf(depth, Code::from_slice(&ev[start..cursor.position()]));
        }
        debug_assert_eq!(
            cursor.position(),
            matched_end,
            "a matched prefix ends on a plateau boundary"
        );
        *self = Out::Built(builder);
    }

    /// Finish the walk's output: the built stream when a plateau
    /// diverged, or `None` for an unchanged walk (every plateau
    /// matched; `fill(i, e) = e`, byte-exact by canonical uniqueness).
    pub(super) fn finish(self, ev: &BitsSlice) -> Option<BitsMut> {
        match self {
            Out::Built(builder) => Some(builder.finish()),
            Out::Verbatim { matched_end } => {
                debug_assert_eq!(
                    matched_end,
                    ev.len(),
                    "an unchanged walk matches every input plateau"
                );
                None
            }
        }
    }
}

/// Grow's route DP riding the fill walk (module doc): the per-branch
/// direction records the splice emit replays, dead from the first
/// divergence on.
pub(super) struct RouteProbe {
    /// The recorded directions; allocated at the first record so a walk
    /// that diverges before any post-order fold pays nothing.
    route: Option<Route>,
    /// The id stream's bit length (the route's key space).
    id_span: usize,
    /// False once the walk diverges: every fold degenerates to the
    /// plain skip and no direction is recorded.
    live: bool,
}

impl RouteProbe {
    pub(super) fn new(id_span: usize) -> Self {
        RouteProbe {
            route: None,
            id_span,
            live: true,
        }
    }

    /// Stop probing: the changed flag tripped, so the route will never
    /// be read.
    pub(super) fn kill(&mut self) {
        self.live = false;
        self.route = None;
    }

    /// Fold a branch node whose children's costs the walk computed
    /// (`grow((il, ir), (n, el, er))`: the cheaper child, ties right,
    /// cost + 1), recording the chosen direction at the branch's id
    /// key.
    pub(super) fn join(&mut self, key: usize, left: Cost, right: Cost) -> Cost {
        if !self.live {
            return COST_MAX;
        }
        let chose_left = left < right;
        self.route().record(key, chose_left);
        let m = if chose_left { left } else { right };
        (m.0, m.1.saturating_add(1))
    }

    /// Fold the leaf-under-internal-id arm (`grow(i, n) = grow(i,
    /// (n, 0, 0)) + N`).
    ///
    /// The whole id subtree below prices one
    /// expansion per level, so the fold reads the skipped id per tag —
    /// the same 2-bit reads `IdReader::skip` pays — computing each
    /// node's distance to its nearest owned terminal and recording the
    /// direction toward it (ties right); it advances `id` past both
    /// children, exactly as the plain skips would.
    pub(super) fn expand(
        &mut self,
        key: usize,
        id: &mut IdReader,
        left: bool,
        right: bool,
    ) -> Cost {
        if !self.live {
            if left {
                id.skip();
            }
            if right {
                id.skip();
            }
            return COST_MAX;
        }
        let l = if left {
            self.expand_subtree(id)
        } else {
            COST_MAX
        };
        let r = if right {
            self.expand_subtree(id)
        } else {
            COST_MAX
        };
        let chose_left = l < r;
        self.route().record(key, chose_left);
        let m = if chose_left { l } else { r };
        (m.0.saturating_add(1), m.1.saturating_add(1))
    }

    /// Take the finished route for the splice emit (the unchanged
    /// branch's epilogue). A walk that recorded nothing (the whole
    /// tree was one pass-through or one owned leaf) hands the splice an
    /// empty route, which it never reads.
    pub(super) fn take_route(&mut self) -> Route {
        debug_assert!(self.live, "the unchanged branch's probe survived the walk");
        self.route
            .take()
            .unwrap_or_else(|| Route::new(self.id_span))
    }

    fn route(&mut self) -> &mut Route {
        self.route.get_or_insert_with(|| Route::new(self.id_span))
    }

    /// The expansion DP over one skipped id subtree.
    ///
    /// Every internal
    /// node costs one expansion and one depth whichever child it
    /// descends, so its cost is `(k, k)` for `k` the distance to its
    /// nearest owned terminal (`COST_MAX` where no child is present —
    /// unreachable in normal form, kept total); the fold records the
    /// direction at every internal node's id key and leaves `id` just
    /// past the subtree.
    ///
    /// Iterative, with the suspended ancestors held as bits — one phase
    /// bit and one right-presence bit per frame, key deltas and the
    /// deferred left distance on the pop-able value stack — so a deep
    /// id spine costs bits of transient per level, not a machine-word
    /// frame (the same discipline as the walk's other bit stacks); the
    /// depth-recursion guard is unneeded because nothing recurses.
    fn expand_subtree(&mut self, id: &mut IdReader) -> Cost {
        // Phase per frame: false = the left child's distance is
        // outstanding, true = the right child's.
        let mut phase = BitStack::new();
        let mut right_present = BitStack::new();
        let mut vals = PopStack::new();
        let mut reg = 0usize;
        // `None`: enter the subtree at the cursor; `Some(d)`: rise with
        // a computed distance (`u32::MAX` infeasible).
        let mut rise: Option<u32> = None;
        loop {
            let mut d = match rise.take() {
                Some(d) => d,
                None => {
                    let key = id.pos();
                    match id.read() {
                        // An owned terminal: the landing, distance 0.
                        IdNode::Full => 0,
                        IdNode::Internal { left, right } => {
                            vals.push((key - reg) as u64);
                            reg = key;
                            phase.push(false);
                            right_present.push(right);
                            if left {
                                continue;
                            }
                            // Left absent: rise its infeasibility into
                            // the frame just pushed.
                            u32::MAX
                        }
                        IdNode::Empty => unreachable!("a present id child is a real node"),
                    }
                }
            };
            // Rise `d` through completed frames.
            loop {
                match phase.last() {
                    None => {
                        return if d == u32::MAX { COST_MAX } else { (d, d) };
                    }
                    Some(false) => {
                        // The left distance arrived: defer it, descend
                        // right (or rise its absence straight back).
                        phase.set_last(true);
                        vals.push(encode_cost_component(d));
                        if right_present.last().expect("one presence bit per frame") {
                            break;
                        }
                        d = u32::MAX;
                    }
                    Some(true) => {
                        // Both children measured: pick, record, fold.
                        phase.pop();
                        right_present.pop();
                        let left = decode_cost_component(vals.pop());
                        let right = d;
                        let key = reg;
                        reg = key - vals.pop() as usize;
                        // Strict `<` ties to the right, matching the
                        // lexicographic (k, k) comparison.
                        let chose_left = left < right;
                        self.route().record(key, chose_left);
                        let m = if chose_left { left } else { right };
                        d = m.saturating_add(1);
                    }
                }
            }
        }
    }
}
