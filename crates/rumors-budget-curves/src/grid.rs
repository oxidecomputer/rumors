//! The pairs and links the curves cover, and the budgets each runs at.

use rumors::MIN_SYNC_MEMORY_BUDGET;
use rumors_testkit::bench::session::{LAN_BDP, LONG_HAUL_BDP, METRO_BDP, Shape, admitting};
use serde::{Deserialize, Serialize};

/// The links the curves cover by default, by bandwidth-delay product: a
/// LAN, a metropolitan link, and the reference long-haul link.
pub const LINKS: [u64; 3] = [LAN_BDP, METRO_BDP, LONG_HAUL_BDP];

/// The smallest power of two the ladder's candidates start from by
/// default: 64 KiB beside the floor and session allowances.
pub const SMALLEST_CANDIDATE: u64 = 1 << 16;

/// One pair of replicas on one link.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Pair {
    /// Messages each replica holds.
    pub messages: u64,
    /// The share of each replica the other holds too, in percent.
    pub shared_percent: u64,
    /// The link's bandwidth-delay product, in bytes.
    pub bdp: u64,
}

impl Pair {
    /// The pair's shape.
    pub fn shape(&self) -> Shape {
        Shape::overlapping(self.messages, self.shared_percent)
    }
}

/// Every combination of `sizes`, `overlaps`, and `links`.
pub fn pairs(sizes: &[u64], overlaps: &[u64], links: &[u64]) -> Vec<Pair> {
    let mut pairs = Vec::new();
    for &messages in sizes {
        for &shared_percent in overlaps {
            for &bdp in links {
                pairs.push(Pair {
                    messages,
                    shared_percent,
                    bdp,
                });
            }
        }
    }
    pairs
}

/// The budgets below `Λ` a pair runs at.
///
/// `Λ` is a ledger's largest excess, what the budget counts, so each rung the
/// ladder derives from it, from `Λ` itself down, is a budget for the ledger
/// to admit work against: the pair runs it at the memory budget that sets
/// that much aside beside the floor and session allowances ([`admitting`]),
/// raised to
/// [`MIN_SYNC_MEMORY_BUDGET`] as a session raises a public budget. So the
/// ladder starts at the minimum: every rung below it runs as the minimum
/// does, and collapses into the minimum's rung. Explicit budgets run as
/// given, below the minimum included, through the test path.
#[derive(Clone, Debug)]
pub struct Ladder {
    /// Rungs per halving: `Λ · 2^(−k / steps_per_octave)`. Zero leaves
    /// out the geometric ladder.
    pub steps_per_octave: u32,
    /// Halvings the geometric ladder spans.
    pub octaves: u32,
    /// Whether to run the minimum's rung,
    /// [`MIN_SYNC_MEMORY_BUDGET`].
    pub minimum: bool,
    /// The smallest power of two to run among the ladder's candidates.
    pub smallest_candidate: u64,
    /// Memory budgets to run instead of all the above, if any, each as
    /// given, below the minimum included.
    ///
    /// They serve a pair whose sessions are too large to run side by side
    /// in one process, where their allocations contend for the process's
    /// heap, so that each budget can run in a process of its own.
    pub explicit: Vec<u64>,
}

impl Ladder {
    /// The memory budgets to run for a pair whose unlimited-budget peak is
    /// `peak`, largest first.
    ///
    /// They are the explicit budgets if any are given, and otherwise the
    /// ladder's rungs, each beside the floor and session allowances, at least
    /// the minimum.
    ///
    /// The rungs are `Λ` itself, the geometric ladder below it, three
    /// quarters, three eighths, and three sixteenths of it, every power of
    /// two from the smallest candidate up that lies below it, and the
    /// minimum.
    pub fn budgets(&self, peak: u64) -> Vec<u64> {
        if !self.explicit.is_empty() {
            let mut budgets = self.explicit.clone();
            budgets.sort_unstable_by(|a, b| b.cmp(a));
            budgets.dedup();
            return budgets;
        }
        let mut budgets = vec![peak];
        let steps = self.steps_per_octave * self.octaves;
        for step in 1..=steps {
            let exponent = -f64::from(step) / f64::from(self.steps_per_octave);
            budgets.push((peak as f64 * exponent.exp2()) as u64);
        }
        if self.steps_per_octave > 0 {
            budgets.extend([peak * 3 / 4, peak * 3 / 8, peak * 3 / 16]);
        }
        budgets.extend(
            (self.smallest_candidate.next_power_of_two().trailing_zeros()..u64::BITS)
                .map(|exponent| 1u64 << exponent)
                .take_while(|&candidate| candidate < peak),
        );
        let minimum = u64::try_from(MIN_SYNC_MEMORY_BUDGET).unwrap_or(u64::MAX);
        let mut budgets: Vec<u64> = budgets
            .into_iter()
            .map(admitting_rung)
            .filter(|&budget| budget > minimum)
            .collect();
        if self.minimum {
            budgets.push(minimum);
        }
        budgets.sort_unstable_by(|a, b| b.cmp(a));
        budgets.dedup();
        budgets
    }
}

/// The memory budget whose ledger admits work against `rung` bytes.
fn admitting_rung(rung: u64) -> u64 {
    let rung = usize::try_from(rung).unwrap_or(usize::MAX);
    u64::try_from(admitting(rung)).unwrap_or(u64::MAX)
}
