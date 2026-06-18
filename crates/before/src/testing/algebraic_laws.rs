//! Oracle-independent algebraic laws, asserted directly on the impl.
//!
//! Every other differential test in this crate keys correctness to the
//! recursive oracle (impl `==` oracle, structurally). That catches any
//! divergence between the two — but it is, by construction, blind to a bug the
//! impl and the oracle make *together* (a shared reference and implementation
//! can be wrong in the same way). The laws here are different: they hold by the
//! ITC algebra (paper §2-§4) *regardless of the reference*, so they pin the
//! impl to the math itself, not to a second implementation of it.
//!
//! Inputs come from the arbitrary-normal-form generators
//! ([`crate::testing::generators::arb_oracle_version`] / `arb_oracle_party`),
//! so the laws are checked on the full space of valid trees — including the
//! large-base (path sums that would overflow `u64`) events — not just the
//! shapes the op pipeline produces. `Party` is `!Clone`, so each use rebuilds a
//! fresh impl `Party` from its oracle tree via `from_oracle_party`; the oracle
//! tree is only a *source of bits*, never an arbiter of the result.

#[cfg(test)]
mod tests;
