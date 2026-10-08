"""The calibration mutants, as anchored reversible swaps.

Usage: python3 -I mutants.py apply|revert ID...

Each mutant is (file, anchor, original, mutated), relative to the before
crate's src/. `apply` swaps original for mutated after the anchor; `revert`
swaps back. Verify restoration with `git diff` afterwards.
"""

import subprocess
import sys

SRC = "/Users/oxide/src/rumors-slot-31/crates/before/src/"
SWAP = "/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/builder-trait-coherence/swap.py"

MUTANTS = {
    # Hash bodies replaced by `()` (the cargo-mutants survivor for Version).
    "M1-version-hash-unit": (
        "version.rs", "impl Hash for Version {",
        "self.0.hash(state);", "let _ = (&self.0, state);"),
    "M2-party-hash-unit": (
        "party.rs", "impl core::hash::Hash for Party {",
        "core::hash::Hash::hash(&self.0, state);", "let _ = (&self.0, state);"),
    # Count: Debug -> Ok(Default), Add<&Count> for Count -> Default, Sum<&Count> -> Default.
    "M3-count-debug-empty": (
        "count.rs", "impl fmt::Debug for Count {",
        "<Self as fmt::Display>::fmt(self, f)", "{ let _ = (self, f); Ok(Default::default()) }"),
    "M4-count-add-ref-default": (
        "count.rs", "impl Add<&Count> for Count {",
        "&self + rhs", "{ let _ = (self, rhs); Count::default() }"),
    "M5-count-sum-ref-default": (
        "count.rs", "impl<'a> Sum<&'a Count> for Count {",
        "iter.fold(Count::ZERO, |mut acc, t| {", "iter.take(0).fold(Count::ZERO, |mut acc, t| {"),
    # Ranked Debug -> Ok(()).
    "M6-ranked-debug-empty": (
        "ranked.rs", "impl core::fmt::Debug for Ranked<'_> {",
        'f.debug_struct("Ranked")\n            .field("version", &self.version)\n            .finish()',
        "{ let _ = (self, f); Ok(Default::default()) }"),
    # Floor Debug renders a different expression than its query.
    "M8-floor-debug-diverges": (
        "causally/forms.rs", "impl fmt::Debug for Floor<'_> {",
        'write!(f, "after({:?})", self.at)', 'write!(f, "floor({:?})", self.at)'),
    # size_hint constants.
    "M9-plateaus-hint-zero": (
        "shape.rs", "impl Iterator for Plateaus<'_> {",
        "(1, None)", "(0, Some(0))"),
    "M10-regions-hint-stale-lower": (
        "shape.rs", "impl Iterator for Regions<'_> {",
        "(0, Some(0))", "(1, None)"),
    "M12-overlay-hint-zero": (
        "shape.rs", "impl Iterator for Overlay<'_> {",
        "(1, None)", "(0, Some(0))"),
    "M15-cells-hint-stale-lower": (
        "shape.rs", "impl<const N: usize> Iterator for Cells<'_, N> {",
        "(0, Some(0))", "(1, None)"),
    "M11-partyforks-hint-zero": (
        "party/forks.rs", "fn size_hint(&self) -> (usize, Option<usize>) {\n        self.plan.size_hint()",
        "self.plan.size_hint()", "(0, Some(0))"),
    "M13-clockforks-hint-zero": (
        "clock/forks.rs", "fn size_hint(&self) -> (usize, Option<usize>) {",
        "self.parties.size_hint()", "(0, Some(0))"),
    # Clock `|=` arm (both `|=` cells) forgets to absorb.
    "M14-clock-assign-skips-absorb": (
        "clock.rs", "(@cell $island:literal, $contract:literal, $opdoc:literal, as_clock",
        "self.absorb(r.borrow());", "let _ = r;"),
    # OwnVersion: argument-order slip in the `&lhs` form's `gt`.
    "M16-ownversion-ref-gt-converse": (
        "version/own.rs", "impl<$($lt),*> PartialOrd<$rhs> for &$lhs {",
        "fn gt(&self, o: &$rhs) -> bool { super::projection::Comparison::lt(o, *self) }",
        "fn gt(&self, o: &$rhs) -> bool { super::projection::Comparison::lt(*self, o) }"),
    # Span: the `Version op &Span` cell drops its span operand.
    "M17-span-version-ref-cell-drops-span": (
        "span/algebra.rs", "fn $op(self, r: &Span<'b>) -> Span<'static> {\n                Span::$method(&Span::at(&self), r)",
        "Span::$method(&Span::at(&self), r)", "Span::$method(&Span::at(&self), { let _ = r; Span::at(&self) })"),
    # From<&Query> drops the holes.
    "M18-query-from-ref-drops-holes": (
        "causally/convert.rs", "impl<'a, P: Polarity> From<&Query<'a, P>> for Query<'a, P> {",
        "query.clone()", "{ let mut copy = query.clone(); copy.holes.clear(); copy }"),
    # Rank: Add<&Rank> for Rank -> ZERO; Debug -> empty.
    "M19-rank-add-ref-zero": (
        "rank.rs", "impl Add<&Rank> for Rank {",
        "&self + rhs", "{ let _ = (self, rhs); Rank::ZERO }"),
    "M20-rank-debug-empty": (
        "rank.rs", "impl Debug for Rank {",
        "<Self as Display>::fmt(self, f)", "{ let _ = (self, f); Ok(Default::default()) }"),
}


def main() -> None:
    mode, ids = sys.argv[1], sys.argv[2:]
    for mutant in ids:
        path, anchor, original, mutated = MUTANTS[mutant]
        old, new = (original, mutated) if mode == "apply" else (mutated, original)
        subprocess.run(["python3", "-I", SWAP, SRC + path, anchor, old, new], check=True)


main()
