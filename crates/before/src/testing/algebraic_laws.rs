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
//! The laws, on the impl's own `Version` / `Party` surface (no oracle on the
//! right-hand side of any assertion):
//!
//! - **merge / meet are a lattice**: each of `|` and `&` is idempotent,
//!   commutative, and associative, with `|` an upper bound (`a <= a | b`) and
//!   `&` a lower bound (`a & b <= a`); **absorption** (`a & (a | b) == a`) and
//!   **distributivity** (`a & (b | c) == (a & b) | (a & c)`, and its dual) tie
//!   them into a distributive lattice — the structure the causal order rests on.
//! - **the causal order is a partial order**: reflexive, antisymmetric
//!   (`a <= b && b <= a ⇒ a == b`), and transitive (`a <= b && b <= c ⇒ a <= c`).
//! - **`rank` is a metric valuation**: `rank(a | b) + rank(a & b) == rank(a) +
//!   rank(b)` (a lattice valuation), which makes `Version::distance` a true
//!   metric — symmetric, separating, and obeying the triangle inequality — and
//!   `Version::lag` its directed half.
//! - **`fork` then `join` round-trips**: splitting a share and rejoining the two halves
//!   recovers the original id (the halves are disjoint, so `join` succeeds and `sum`
//!   reconstructs the whole).
//! - **`split` ⊕ `sum` disjointness**: the two halves a `fork` produces are disjoint
//!   (`is_disjoint`), and neither is the anonymous id.
//! - **balanced fork agrees with itself**: the consuming `From<Party>` for
//!   `[Party; N]` split equals the residual `forks(N - 1)` keeps followed by the
//!   shares it yields (one balanced split, one preorder), and dropping `forks`
//!   early folds the untaken shares back into the borrowed party.
//! - **`join_all` is a lossless best-effort fold**: it reunites a fork back to
//!   the original id, is a no-op on the empty iterator (`self` seeds the fold),
//!   and on overlap absorbs every disjoint share while handing back exactly the
//!   ones that clashed — never dropping a party.
//! - **`decode ∘ encode == id`** on `Party` and `Version` (the codec is a section of
//!   the canonical byte form; required for byte-equality `Eq`/`Hash` to be sound).
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
