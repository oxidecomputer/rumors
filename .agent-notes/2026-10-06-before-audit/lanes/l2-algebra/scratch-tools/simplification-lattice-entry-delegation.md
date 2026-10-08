# Simplification brief: one implementation per lattice operator

Kind: self-contained. It preserves behavior, changes no public API or format,
and touches only `crates/before/src/version.rs`.

## Current code

The macro comment above `binop_matrix!` (`version.rs:1375-1385`) says the
borrowed assignment operator "is the single implementation of each operator
family". Two named methods repeat that implementation instead of delegating
to it:

- `Version::join` (`version.rs:510-522`) runs its own short-circuit ladder:
  `other` empty, then `self` empty, then `self == other`, then
  `lattice::Extreme::Higher.emit(self, other)`. This duplicates
  `BitOrAssign<&Version>::bitor_assign` (`version.rs:1453-1467`).
- `Version::meet` (`version.rs:591-603`) duplicates
  `BitAndAssign<&Version>::bitand_assign` (`version.rs:1473-1487`) the same
  way.
- `Version::balanced_fold` (`version.rs:886-918`) takes two function
  parameters, `combine` (the named method) and `fold_view` (the assigning
  form). The two describe the same operation.

I compared the two ladders arm by arm. For every input pair they return the
same value, and they share a buffer in the same cases:

- join: an empty `other` gives `self`; an empty `self` gives `other`; equal
  operands give `self`; anything else goes to `Higher.emit`.
- meet: an empty `self` gives `self`; an empty `other` gives `new()`; equal
  operands give `self`; anything else goes to `Lower.emit`.

## Proposed structure

- `pub fn join(&self, other: &Version) -> Version { self | other }`, and
  `meet` likewise with `&`. Both reach the `&Version, &Version, clone` cell:
  an `O(1)` shared-buffer clone, then the borrowed assignment. Keep each
  method's rustdoc and `#[must_use]` unchanged.
- Drop `balanced_fold`'s `combine` parameter. Its `(Group::Input(a),
  Group::Input(b))` arm becomes
  `{ let mut out = a.borrow().clone(); fold_view(&mut out, b.borrow()); out }`.
  The `join_all`, `meet_all`, and both `Sum` call sites then pass only the
  assigning closure.

## Why the result is more obviously correct

Each lattice direction then has exactly one short-circuit ladder and one call
into the kernel, as the macro comment already claims. A future change to the
fast paths, such as a new shared-storage shortcut, cannot leave the named
method and the operator disagreeing. Today only the laws
`join_method_is_the_operator` and `meet_method_is_the_operator` would catch
such a divergence. After the change they hold by construction and remain as
regression guards.

## Coverage

- `testing::laws` `VERSION_PAIR`: `join_method_is_the_operator`,
  `meet_method_is_the_operator`, and the commutativity, absorption, and
  bound laws. `VERSION_LIST` and `VERSION_AND_LIST`:
  `version_sum_is_the_sequential_pair_fold`,
  `join_all_is_the_sequential_pair_fold`, `meet_all_is_the_sequential_pair_fold`.
- `src/version/lattice/tests.rs` (`assert_emits` calls `join` and `meet`
  directly against the recursive oracle).
- The board rows `version_join`, `version_meet`, `version_join_all`, and
  `version_meet_all` should read identically: the clone is a refcount bump,
  with no heap growth, scan, or digit touch. The builder confirms this with
  the board leg of `just gate`.
- My explore-branch probe (`crates/before/tests/l2_probe`) checks every
  operator cell and the named methods against an independent model.
