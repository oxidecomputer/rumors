//! Two estimates of a session's duration at a budget below its
//! unlimited-budget peak, from the unlimited session alone.
//!
//! They are a lower estimate from the bytes questions keep
//! ([`Unlimited`]), and the profile estimate from the budget-counted bytes
//! over time ([`Profile`]).
//!
//! # The derivation
//!
//! Time is in byte-times on the link model ([`crate::bench::link`]): the
//! link carries one byte per byte-time in each direction, and its round
//! trip `D` is numerically its bandwidth-delay product.
//!
//! Each side admits every question it asks with a *grant*, the most the
//! question's items and the peer's reply can hold, and builds the question
//! out of it. What the grant does not cover once the question is built
//! returns at once; for most questions that is nearly all of it, since a
//! grant prices a full fan of reactions and a typical chunk holds a few.
//! What the question *keeps* (its built items and the room set aside for
//! the reply) stays charged until the side has finished the reply, at
//! least a round trip later: the question crosses to the peer, and the
//! reply crosses back.
//!
//! So each kept byte occupies the budget for about a round trip. A side
//! whose questions keep `R` bytes in all, under a budget of `B`, can
//! sustain no more than `B` such bytes per round trip, and needs at least
//! `R · D / B` byte-times to ask them. The busier side's total `R*` binds.
//! And a session at a budget below its unlimited-budget peak runs at best
//! about as fast as the unlimited session: measured sessions there have run
//! faster than it only by small amounts, where a different budget reorders
//! the admission of their work. Together, up to those small inversions:
//!
//! ```text
//! T(B) ≥ max(T∞, R* · D / B)
//! ```
//!
//! That is: a budget below `R* · D / T∞` slows the session, in inverse
//! proportion to the budget.
//!
//! # What it claims, and where it is loose
//!
//! It is a lower estimate, not a prediction. The two premises are measured
//! rather than proved, and the first holds only approximately: that a
//! session below its unlimited-budget peak runs no faster than with no
//! budget, which measured sessions break by small amounts, and that a kept
//! byte stays charged for a round trip (a question's chunk returns once its
//! last frame is written, which can be sooner). Measured sessions run slower than it
//! predicts in three regimes:
//!
//! - **Just below the unlimited-budget peak, on a link whose round trip
//!   dominates the session,** the session loses whole round trips at once:
//!   a question that waits for budget waits a round trip, which a session
//!   of a few round trips feels as a large fraction.
//! - **Below the total the responder's questions about the root's children
//!   keep,** those questions cannot all be outstanding together. Admission
//!   takes deeper units first, so once deeper work fills the budget the
//!   rest are admitted only within their kind and depth's bound, a few at a
//!   time, each costing the round trips of the descent beneath it. The slowdown rises steeply
//!   there, on every link.
//! - **On a link whose bandwidth dominates the session,** grants stay
//!   charged for as long as the replies ahead of theirs take to cross, not
//!   for one round trip, so the budget that keeps the link busy is far
//!   above `R* · D / T∞`.
//!
//! The inputs come from the unlimited session of the same pair and link:
//! its duration, and each side's question grants and what they returned,
//! as its ledger's admissions report them.
//!
//! # The profile estimate
//!
//! The unlimited session's ledgers swell, peak, and drain. Write `m(t)` for
//! the larger of the two sides' budget-counted bytes at time `t` of that
//! session ([`LedgerReading::profile`]). Under a budget `B`, the ledger's:
//! what a memory budget leaves its ledger beside the floor allowance, which
//! its kinds and depths of unit hold within their bounds outside the
//! budget, and the session allowance, its fixed state and wire pools. Every budget the public setter reaches gives
//! the ledger at least the largest grant, so `B` is positive and the
//! estimate finite wherever a user can run a session. Suppose
//!
//! 1. the session does the same work, in the same order, as the unlimited
//!    one;
//! 2. a stretch of that work that held `m(t)` bytes for `dt` needs
//!    `m(t) · dt` of memory × time wherever it runs, so a budget of `B`
//!    carries it at a rate of at most `B / m(t)`: it takes `dt · m(t) / B`
//!    once `m(t)` exceeds `B`;
//! 3. a stretch that held no more than `B` runs as fast as it did.
//!
//! Then
//!
//! ```text
//! T(B) ≈ ∫ max(1, m(t) / B) dt   over the unlimited session
//!      = T∞ + ∫ (m(t) − B)⁺ dt / B
//! ```
//!
//! That is: each moment of the unlimited session is stretched by how far
//! its memory exceeds the budget. The larger side sets the stretch, since
//! the two sides advance together and each has a budget of its own.
//!
//! It is never below `max(T∞, A / B)`, where `A = ∫ m(t) dt` is the
//! unlimited session's budget area, since `max(1, x)` is at least both 1
//! and `x`; the two agree where `m(t)` stays above `B` throughout. The
//! area estimate lets a moment's excess spill into moments below the
//! budget; the profile estimate does not.
//!
//! It is an estimate, not a bound, in either direction:
//!
//! - **Not a lower bound.** A budgeted session need not repeat the
//!   unlimited one's schedule: much of what the unlimited session holds is
//!   held only because nothing stopped it (replies parked behind a busy
//!   stage, questions asked far ahead of their replies), and a budgeted
//!   session can do the same work holding less, for no longer.
//! - **Not an upper bound.** Premise 2 holds memory × time constant per
//!   stretch of work, but a waiting unit can hold the work behind it in
//!   its stage while everything already admitted keeps its charges, so a
//!   budgeted session can hold its budget full for far longer than the
//!   unlimited session held the same bytes. Waits also cost round trips,
//!   which no memory accounts for.
//!
//! Where measured sessions run far above it, the budget is held by bytes
//! the unlimited session held only briefly.
//!
//! [`LedgerReading::profile`]: crate::bench::session::LedgerReading::profile
//! [`Unlimited`]: crate::bench::slowdown::Unlimited
//! [`Profile`]: crate::bench::slowdown::Profile

use crate::bench::session::{Session, Step};

/// What the estimate reads from a session run with an unlimited budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Unlimited {
    /// The session's duration, `T∞`, in byte-times.
    pub duration: u64,
    /// The link's round trip, `D`, in byte-times.
    pub rtt: u64,
    /// The larger of the two sides' totals of the bytes their questions
    /// kept out of their grants, `R*`: what a question holds once it is
    /// built, while its reply crosses the link.
    pub question_bytes: u64,
}

impl Unlimited {
    /// Reads the estimate's inputs from `session`, which ran with an
    /// unlimited budget.
    ///
    /// # Panics
    ///
    /// If any unit of `session` waited for admission, which no unit does
    /// under an unlimited budget.
    pub fn of(session: &Session) -> Self {
        assert_eq!(
            session.admitted.waited(),
            0,
            "the estimate reads a session no budget constrained",
        );
        Self {
            duration: session.duration(),
            rtt: session.case.link.rtt(),
            question_bytes: session
                .admitted
                .ledgers
                .iter()
                .map(|ledger| ledger.all_questions().kept())
                .max()
                .unwrap_or(0),
        }
    }

    /// The budget below which the estimate has the session slow down,
    /// `R* · D / T∞`, in bytes.
    pub fn knee(&self) -> f64 {
        self.question_bytes as f64 * self.rtt as f64 / self.duration as f64
    }

    /// The estimated least duration at `budget` bytes, in byte-times.
    ///
    /// A budget of zero has no finite estimate: there each kind and depth's
    /// bound, not the budget, sets the pace, which the estimate does not
    /// model.
    pub fn duration_at(&self, budget: u64) -> f64 {
        let paced = self.question_bytes as f64 * self.rtt as f64 / budget as f64;
        paced.max(self.duration as f64)
    }

    /// The estimated least duration at `budget` bytes as a multiple of the
    /// unlimited session's.
    pub fn slowdown_at(&self, budget: u64) -> f64 {
        self.duration_at(budget) / self.duration as f64
    }
}

/// The profile estimate's input: the unlimited session's duration and the
/// larger side's budget-counted bytes over it ([module docs](self)).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    /// The session's duration, `T∞`, in byte-times.
    pub duration: u64,
    /// The larger side's budget-counted bytes, `m(t)`, as steps in time
    /// order: each holds from its time until the next step's, the last
    /// until `duration`. Before the first step, `m(t)` is zero.
    pub steps: Vec<Step>,
}

impl Profile {
    /// Reads the estimate's input from `session`, which ran with an
    /// unlimited budget.
    ///
    /// # Panics
    ///
    /// If any unit of `session` waited for admission, which no unit does
    /// under an unlimited budget.
    pub fn of(session: &Session) -> Self {
        assert_eq!(
            session.admitted.waited(),
            0,
            "the estimate reads a session no budget constrained",
        );
        let sides: Vec<&[Step]> = session
            .admitted
            .ledgers
            .iter()
            .map(|ledger| ledger.profile.as_slice())
            .collect();
        Self::of_sides(session.duration(), &sides)
    }

    /// The profile of a session of `duration` byte-times whose sides held
    /// `sides`, each a profile as [`LedgerReading::profile`] records it:
    /// at every moment, the larger side's bytes.
    ///
    /// [`LedgerReading::profile`]: crate::bench::session::LedgerReading::profile
    pub fn of_sides(duration: u64, sides: &[&[Step]]) -> Self {
        let mut events: Vec<(u64, usize, usize)> = sides
            .iter()
            .enumerate()
            .flat_map(|(side, steps)| steps.iter().map(move |step| (step.at, side, step.bytes)))
            .collect();
        // Stable, so each side's steps keep their order within one time.
        events.sort_by_key(|&(at, _, _)| at);
        let mut current = vec![0; sides.len()];
        let mut steps: Vec<Step> = Vec::new();
        for (index, &(at, side, bytes)) in events.iter().enumerate() {
            current[side] = bytes;
            // Only the value once every change at a time is made holds for
            // any time at all.
            if events.get(index + 1).is_some_and(|next| next.0 == at) {
                continue;
            }
            let larger = current.iter().copied().max().unwrap_or(0);
            if steps.last().is_none_or(|last| last.bytes != larger) {
                steps.push(Step { at, bytes: larger });
            }
        }
        Self { duration, steps }
    }

    /// Each step's bytes and how long they held, in byte-times.
    fn spans(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        let ends = self
            .steps
            .iter()
            .skip(1)
            .map(|step| step.at)
            .chain(std::iter::once(self.duration));
        self.steps
            .iter()
            .zip(ends)
            .map(|(step, end)| (step.bytes as u64, end.saturating_sub(step.at)))
    }

    /// The budget area of the larger side at every moment, `∫ m(t) dt`, in
    /// bytes × byte-times: at least either side's own.
    pub fn area(&self) -> u128 {
        self.spans()
            .map(|(bytes, span)| u128::from(bytes) * u128::from(span))
            .sum()
    }

    /// The most bytes `m(t)` reaches: the budget from which up the
    /// estimate is `T∞`.
    pub fn peak(&self) -> u64 {
        self.spans().map(|(bytes, _)| bytes).max().unwrap_or(0)
    }

    /// The estimated duration at `budget` bytes, in byte-times:
    /// `T∞ + ∫ (m(t) − B)⁺ dt / B`.
    ///
    /// A budget of zero has no finite estimate: there each kind and depth's
    /// bound, not the budget, sets the pace, which the estimate does not
    /// model.
    pub fn duration_at(&self, budget: u64) -> f64 {
        if budget == 0 {
            return f64::INFINITY;
        }
        let excess: u128 = self
            .spans()
            .map(|(bytes, span)| u128::from(bytes.saturating_sub(budget)) * u128::from(span))
            .sum();
        // The whole and fractional parts of the quotient separately, so the
        // conversion rounds only once, however large the excess.
        let budget = u128::from(budget);
        self.duration as f64 + (excess / budget) as f64 + (excess % budget) as f64 / budget as f64
    }

    /// The estimated duration at `budget` bytes as a multiple of the
    /// unlimited session's.
    pub fn slowdown_at(&self, budget: u64) -> f64 {
        self.duration_at(budget) / self.duration as f64
    }
}

#[cfg(test)]
mod tests;
