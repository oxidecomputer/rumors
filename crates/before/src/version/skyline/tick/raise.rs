//! Raises one owned region when simplification cannot record a tick.
//!
//! The main tick walk chooses the cheapest place to raise. Its cost first
//! minimizes the number of new version branches, then the depth; ties choose
//! the right child. That walk records the chosen direction at each party
//! branch in a [`Route`], so applying the raise does not repeat the search.
//!
//! [`Raise`] replays the route in three phases:
//!
//! 1. Descend to the chosen version leaf, copying complete subtrees that occur
//!    before it and retaining which right subtrees must be copied afterward.
//! 2. Raise the leaf by all requested ticks. If the party continues below that
//!    leaf, expand it into the required subtree and emit the new sibling leaves.
//! 3. Copy the retained right subtrees and finish the canonical version.
//!
//! Version leaves after the first store differences from their predecessor.
//! Raising one leaf therefore changes its own payload and, when it remains the
//! predecessor, the next leaf's payload. Every other off-route subtree is
//! copied unchanged. Each source subtree is scanned once, so work and transient
//! storage are proportional to the party, version, and required output.
//!
//! Applying `k` ticks together is equivalent to applying one tick `k` times.
//! Once the selected region is a version leaf, raising it leaves the topology
//! and the route's costs unchanged. If the first tick must expand a leaf, the
//! new version leaf at the selected party terminal becomes the unique choice
//! needing no expansion, so every remaining tick selects it again.

use core::cmp::Ordering;
use core::ops::Range;

use num_bigint::{BigInt, BigUint, Sign};

use crate::codec::{self, gamma, BitStack, BitsBuf, BitsView};

use super::super::build::{PayloadBuilder, SkylineBuilder};
mod cursor;
mod probe;
mod route;

use cursor::{PartyTags, VersionCursor};
pub(super) use probe::RaiseProbe;
pub(crate) use route::Cost;
pub(super) use route::Route;

/// The boundary repair a spliced subtree's first payload code needs.
#[derive(Clone, Copy)]
enum Repair<'a> {
    /// The predecessor leaf is unchanged: copy the code verbatim.
    None,
    /// The predecessor is the selected leaf, raised by the tick count: the
    /// delta drops by the same amount.
    Minus(&'a BigUint),
}

/// The three height adjustments [`Step::write`] applies.
///
/// The selected leaf's height increases, changing either an absolute payload
/// or a delta. Its successor, if any, needs the opposite delta adjustment to
/// retain its own height. That successor cannot be the first leaf, so an
/// absolute payload never needs a downward adjustment.
#[derive(Clone, Copy)]
enum Step {
    /// The selected first leaf's absolute height rises by the tick count.
    UpAbsolute,
    /// The selected leaf's delta rises by the tick count.
    UpDelta,
    /// The successor's delta falls by the tick count, preserving its height.
    DownDelta,
}

impl Step {
    /// Adjust a delta, reusing its magnitude and borrowing the tick count where
    /// possible.
    fn adjust_delta(self, delta: BigInt, ticks: &BigUint) -> BigInt {
        let direction = match self {
            Step::UpDelta => Sign::Plus,
            Step::DownDelta => Sign::Minus,
            Step::UpAbsolute => unreachable!("an absolute payload is not a delta"),
        };
        let (sign, mut magnitude) = delta.into_parts();
        if sign == Sign::NoSign {
            return BigInt::from_biguint(direction, ticks.clone());
        }
        if sign == direction {
            magnitude += ticks;
            return BigInt::from_biguint(sign, magnitude);
        }
        match magnitude.cmp(ticks) {
            Ordering::Greater => BigInt::from_biguint(sign, magnitude - ticks),
            Ordering::Less => BigInt::from_biguint(direction, ticks - magnitude),
            Ordering::Equal => BigInt::from(0u8),
        }
    }

    /// Re-code one payload after applying this height adjustment.
    fn write(
        self,
        out: &mut PayloadBuilder<'_>,
        bits: BitsView<'_>,
        code: Range<u64>,
        ticks: &BigUint,
    ) {
        let (value, end) = codec::gamma::decode(bits, code.start).expect("canonical skyline bits");
        debug_assert_eq!(end, code.end, "a payload range is exactly one code");
        match self {
            Step::UpAbsolute => {
                gamma::encode(&(value + ticks), out);
                return;
            }
            Step::UpDelta | Step::DownDelta => {}
        }
        let delta = self.adjust_delta(gamma::decode_signed(value), ticks);
        gamma::encode_signed(&delta, out);
    }
}

/// The version leaf selected by a raise route.
struct Target {
    /// The selected leaf's payload code in the source version.
    code: Range<u64>,
    /// Party directions below that leaf, where new version branches are needed.
    expansion: BitsBuf,
}

/// State shared by the descent, raise, and unwind phases.
struct Raise<'v, 'p> {
    /// The source version, retained so unchanged payloads can be spliced.
    version_bits: BitsView<'v>,
    /// Forward cursor through version topology and payloads.
    version: VersionCursor<'v>,
    /// Forward cursor through party tags.
    party: PartyTags<'p>,
    /// Canonical output under construction.
    output: SkylineBuilder,
    /// Chosen-path directions; `true` means a right subtree awaits unwind.
    pending: BitStack,
    /// Depth of the selected version leaf.
    depth: u64,
    /// Whether output already contains a leaf before the selected one.
    emitted_before: bool,
}

impl<'v, 'p> Raise<'v, 'p> {
    /// Create a raise over one validated version and owning party.
    fn new(version_bits: BitsView<'v>, party_bits: BitsView<'p>) -> Self {
        Self {
            version_bits,
            version: VersionCursor::new(version_bits),
            party: PartyTags::new(party_bits),
            // Expansion adds at most one short payload per party level. The
            // selected leaf and its successor may be as wide as `ticks`, so
            // this is only a useful starting capacity; the builder can grow.
            output: SkylineBuilder::with_capacity(version_bits.len() + party_bits.len() + 64),
            pending: BitStack::new(),
            depth: 0,
            emitted_before: false,
        }
    }

    /// Replay `route`, apply all `ticks`, and return the canonical version.
    fn run(mut self, route: &Route, ticks: &BigUint) -> BitsBuf {
        let target = self.descend(route);
        let selected_is_predecessor = self.raise_target(target, ticks);
        self.copy_trailing_subtrees(ticks, selected_is_predecessor);
        debug_assert_eq!(
            self.version.pos(),
            self.version_bits.len(),
            "raising consumes the version"
        );
        self.output.finish()
    }

    /// Descend to the selected leaf, copying everything that precedes it.
    fn descend(&mut self, route: &Route) -> Target {
        loop {
            let (key, left_present, right_present) = self.party.read();
            if !left_present && !right_present {
                // An owned terminal raises its corresponding version leaf in
                // place. A version branch here would have been simplified by
                // the deciding walk, so it cannot reach this phase.
                let code = self
                    .version
                    .read()
                    .expect("an owned terminal covers one unsimplified leaf");
                return Target {
                    code,
                    expansion: BitsBuf::new(),
                };
            }

            match self.version.read() {
                None => self.descend_version_branch(route, key, left_present, right_present),
                Some(code) => {
                    return Target {
                        code,
                        expansion: self.collect_expansion(route, key, left_present, right_present),
                    };
                }
            }
        }
    }

    /// Follow one route step where both the party and version branch.
    fn descend_version_branch(
        &mut self,
        route: &Route,
        key: u64,
        left_present: bool,
        right_present: bool,
    ) {
        if route.descends_left(key) {
            debug_assert!(left_present, "a raise route enters an owned child");
            // The right version subtree follows the selected left child. The
            // pending bit is enough to find it later because the version cursor
            // reaches it naturally while unwinding.
            self.pending.push(true);
        } else {
            debug_assert!(right_present, "a raise route enters an owned child");
            // The left version subtree precedes the selected right child and
            // must therefore be emitted now. The matching party subtree is
            // skipped so both cursors arrive at their right children.
            self.version
                .feed_subtree(&mut self.output, self.depth + 1, Repair::None);
            if left_present {
                self.party.skip_subtree();
            }
            self.emitted_before = true;
            self.pending.push(false);
        }
        self.depth += 1;
    }

    /// Follow party branches below one version leaf.
    ///
    /// These directions describe the new version branches required to isolate
    /// the selected owned terminal. Off-route party subtrees need no version
    /// cursor because they all retain the original leaf's height.
    fn collect_expansion(
        &mut self,
        route: &Route,
        key: u64,
        left_present: bool,
        right_present: bool,
    ) -> BitsBuf {
        let mut expansion = BitsBuf::new();
        let mut current = (key, left_present, right_present);
        loop {
            let (key, left_present, right_present) = current;
            let left = route.descends_left(key);
            debug_assert!(
                if left { left_present } else { right_present },
                "a raise route enters an owned child"
            );
            if !left && left_present {
                self.party.skip_subtree();
            }
            expansion.push(left);

            let next = self.party.read();
            if !next.1 && !next.2 {
                return expansion;
            }
            current = next;
        }
    }

    /// Replace the selected leaf with its raised value and any new branches.
    ///
    /// Returns whether the raised leaf is the last emitted leaf. Only in that
    /// case is it the predecessor whose successor delta must be repaired.
    fn raise_target(&mut self, target: Target, ticks: &BigUint) -> bool {
        let expansion_depth = target.expansion.len();
        debug_assert_eq!(
            self.depth,
            self.pending.len(),
            "one pending direction per version level"
        );

        // A rightward expansion has a fresh left sibling before the selected
        // leaf. The first such sibling preserves the source leaf's payload;
        // later siblings have the same height and therefore a zero delta.
        let mut emitted_in_expansion = false;
        for level in 0..expansion_depth {
            if !target.expansion.get(level) {
                if emitted_in_expansion {
                    self.output.leaf(self.depth + level + 1, |out| {
                        gamma::encode(&BigUint::ZERO, out)
                    });
                } else {
                    self.output.leaf(self.depth + level + 1, |out| {
                        out.splice(self.version_bits, target.code.start, target.code.end)
                    });
                }
                emitted_in_expansion = true;
            }
        }

        if emitted_in_expansion {
            // A fresh predecessor retains the original height, so the selected
            // leaf's delta is exactly the positive tick count.
            self.output.leaf(self.depth + expansion_depth, |out| {
                gamma::encode_positive(ticks, out)
            });
        } else {
            let step = if self.emitted_before {
                Step::UpDelta
            } else {
                Step::UpAbsolute
            };
            self.output.leaf(self.depth + expansion_depth, |out| {
                step.write(out, self.version_bits, target.code.clone(), ticks)
            });
        }

        // Leftward expansion creates right siblings after the selected leaf.
        // The nearest one's delta is `-ticks`; later siblings again have zero
        // deltas because their heights match one another.
        let mut selected_is_predecessor = true;
        for level in (0..expansion_depth).rev() {
            if target.expansion.get(level) {
                self.output.leaf(self.depth + level + 1, |out| {
                    if selected_is_predecessor {
                        gamma::encode_negative(ticks, out)
                    } else {
                        gamma::encode(&BigUint::ZERO, out)
                    }
                });
                selected_is_predecessor = false;
            }
        }
        selected_is_predecessor
    }

    /// Copy right subtrees retained during descent.
    fn copy_trailing_subtrees(&mut self, ticks: &BigUint, selected_is_predecessor: bool) {
        // The first following leaf needs `-ticks` only when the selected leaf
        // remains its predecessor. Once one subtree is copied, every later
        // boundary has its original predecessor and can be copied verbatim.
        let mut repair = if selected_is_predecessor {
            Repair::Minus(ticks)
        } else {
            Repair::None
        };
        for level in (0..self.depth).rev() {
            let selected_left = self
                .pending
                .pop()
                .expect("one pending direction per version level");
            if selected_left {
                self.version
                    .feed_subtree(&mut self.output, level + 1, repair);
                repair = Repair::None;
            }
        }
    }
}

#[cfg(test)]
mod tests;
