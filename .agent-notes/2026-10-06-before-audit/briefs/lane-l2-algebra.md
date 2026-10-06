<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the lane section for the L2 auditor, modeled on the L6 codec lane; under review by Finch before launch. -->

# Lane L2: Version algebra

## Scope

This lane's theme is the `Version` *lattice and partial order*. A version
denotes a step function from `[0, 1)` to the naturals, ordered pointwise:
join and meet are pointwise maximum and minimum, normalized to canonical
form, and comparison is pointwise `<=`. Everything here should agree with that
function semantics, on every canonical input, including versions no history
produces.

Some things to consider, as starting points rather than a checklist. They
are prompts for your own exploration: pursue whatever else the code
suggests, and expect the most valuable questions to be ones not listed
here.

- Does every result agree with the function semantics, and is every result
  in canonical form?
- Many paths answer the same question: the full relation, the directional
  operators, equality, `concurrent`, the fast paths for shared storage, and
  the projected and masked comparisons. Do they always agree?
- Do the n-ary folds (`_all`, `Sum`, `FromIterator`, owned and borrowed)
  agree with pairwise application in every order and grouping?
- Is projection exactly restriction to a party's region?
- Do the shape iterators render the step function faithfully?

These are the crate's most heavily optimized kernels (cursor traversals,
re-anchoring, range minima), so their complexity claims belong to this theme
as much as their answers do.

The identifiers below are a starting map, not a boundary. Follow the theme
wherever the code takes it, including into code this list does not name.

- **Lattice:** `join`, `meet`, `span`, their `_all` forms, the `|`, `&`, `^`
  operators and their assigning forms, `Sum` and `FromIterator`.
- **Order:** `PartialEq`, `PartialOrd`, `concurrent`, `Hash`, `is_empty`,
  `new`/`Default`.
- **Projection:** `project`, `/`, `OwnVersion`.
- **Shape:** `Version::shape` (`Plateaus`), `shape::combine` (`Cells`).
- **Implementation:** `src/version/` (lattice, order, projection, own,
  overlay, shape, place, range minima), `src/fold.rs`, `src/shape.rs`.

Neighboring lanes explore adjacent themes. When your work crosses into one,
establish what you found and report it, rather than continuing the
investigation there. L3 explores events and `min_ticks`, L4 the measures
(`rank`, `distance`, `lag`), and L5 spans. L7 examines whether this lane's
calls into the `suanpan` accumulator respect its contract.

## Contracts to read first

- `Version`'s type docs, including the three comparison costs and the
  statement that byte equality is exactly causal equality.
- `join` and `meet`: "The result's canonical encoding is no longer than the
  two operands' encodings together."
- `meet_all` on an empty iterator, and why there is no `Sum` for meet.
- `span`/`span_all`: the tightest span enclosing every `v` with
  `meet <= v <= join`.
- `project` and `OwnVersion`'s docs.

## Prior coverage to map

- **Tests:** `src/version/tests.rs`, `lattice/tests.rs`, `order/tests.rs`,
  `projection/tests.rs`, `own/tests.rs`, `place/tests.rs`,
  `place/filter/tests.rs`, `range_minima/tests.rs`, `src/shape/tests.rs`,
  `tests/fold_skeleton.rs`, the version law modules, and the differential
  table.
- **September partitions:** `version-core.md`, `skyline-query.md`,
  `skyline-sweep-place-masked.md`, `skyline-coding.md`.

## Closed fixes to re-attack

- Join's re-anchoring cascade on wide-leaf, deep-spine combinations.
- Projected and masked comparisons rescanning a parked cursor's trailing run.
- Directional comparisons' early exit.
- The balanced folds' intermediate-size argument.

## Candidate leads (evaluate, don't confirm)

1. **The lattice is distributive and canonical.** Over arbitrary canonical
   versions, including large heights and deep trees, join and meet satisfy
   the lattice laws and distributivity, and every result is in canonical
   form.
2. **Encoding subadditivity.** Construct the worst case for
   `|join(a, b)| <= |a| + |b|` and the same for meet, instead of sampling.
3. **All comparison paths agree.** Full, directional, and equality
   comparisons, `concurrent`, and every operator form agree on every pair,
   including pairs sharing storage.
4. **Folds are order-independent.** Every fold entry (`_all`, `Sum`,
   `FromIterator`, owned and borrowed, with duplicate runs) equals every
   pairwise fold order and grouping.
5. **Projection is restriction.** `v / p` equals `v` restricted to `p`'s
   region, and comparing projections agrees with comparing restricted
   versions.
