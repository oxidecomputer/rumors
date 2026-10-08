# Simplification brief: delete the uncalled `Sealed::hole_subtracts`

Kind: self-contained, behavior-preserving. No public API, wire, or storage
change (the trait is sealed and crate-private in effect).

## Current code

`crates/before/src/causally/polarity.rs`:

- line 40-41: the trait method
  `fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool;` with doc
  "Whether `hole` subtracts `probe`."
- lines 70-76: the `Down` implementation (`probe < at` when strict, else
  `probe <= at`).
- lines 128-134: the `Up` implementation (`at < probe` when strict, else
  `at <= probe`).
- lines 182-184: the `Neutral` implementation (`unreachable!`).

## Evidence that it is dead

- Verified: `grep -rn hole_subtracts crates/` lists only the declaration and
  the three implementations: no caller in production code or tests.
- Verified: branch coverage of the full `before` and `suanpan` suites
  (coverage/before__causally__polarity.txt) records lines 70-76 and 128-134
  as never executed.
- Query evaluation decides membership through `hole_demand` and the fused
  walks' `Demand` values instead; the method is a leftover of an earlier
  evaluation strategy (inferred from its shape; I did not dig through
  history, which is irrelevant to the change).

Rust's dead-code lint does not report unused trait methods, so nothing warns.

## Proposed structure

Delete the declaration and its three implementations. Nothing else changes:
`Hole`'s doc comment (lines 12-17) already states what a hole subtracts in
each polarity, independent of this method.

## Why the result is more obviously correct

The sealed trait then lists exactly the operations query evaluation uses.
Today a reader auditing `Down`/`Up` duality checks a method whose answer
nothing consumes; a future maintainer could also "fix" a polarity bug in it
and believe the fix took effect.

## Coverage

The method has no callers, so no property covers it and none needs to. The
causal query suites (`causally/tests.rs`, the `version_party_*` and
span laws) cover the behavior the remaining methods implement. Verification:
`cargo clippy -p before --all-targets --all-features -- -D warnings`, the
`before` suite, and the gate.
