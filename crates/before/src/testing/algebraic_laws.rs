//! Oracle-independent algebraic laws, asserted directly on the impl.
//!
//! Every other differential test in this crate keys correctness to the
//! recursive oracle (impl `==` oracle, structurally). That catches any
//! divergence between the two — but it is, by construction, blind to a bug the
//! impl and the oracle make *together* (a shared reference and implementation
//! can be wrong in the same way). The laws are different: they hold by the
//! ITC algebra (paper §2-§4) *regardless of the reference*, so they pin the
//! impl to the math itself, not to a second implementation of it.
//!
//! The laws themselves live in [`crate::laws`] as named predicates, grouped
//! by signature; this suite is one of their consumers. [`tests`] holds one
//! generic driver per signature group, each iterating its whole slice so a
//! failure names the violated law, over two input regimes:
//!
//! - **Arbitrary normal forms**: the [`crate::testing::generators`]
//!   generators (`arb_oracle_version` / `arb_oracle_party_nonempty`) cover
//!   the full space of valid trees — including the large-base events (path
//!   sums that would overflow `u64`) — not just the shapes the op pipeline
//!   produces. `Party` is `!Clone`, so each use rebuilds a fresh impl value
//!   from its oracle tree via the bridge; the oracle tree is only a *source
//!   of bits*, never an arbiter of the result. Clocks pair an arbitrary
//!   party with an arbitrary version through `Clock::from_parts` — every
//!   canonical pairing is a valid clock, including ones no op sequence
//!   reaches.
//! - **Organic op-trace populations**: [`crate::testing::optrace`] histories
//!   land the same laws on the value shapes real fork/tick/join/sync
//!   schedules produce.
//!
//! The third consumer is the fuzz workspace's law target, which drives the
//! same slices over decoded hostile-but-canonical values.

#[cfg(test)]
mod tests;
