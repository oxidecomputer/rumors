# Simplification brief: decide the crossed clamp by `floor <= ceiling`

Kind: constant-factor improvement, self-contained (no public API, format, or
semantic change). Files: `crates/before/src/causally/query.rs`, and
`crates/before/src/causally/polarity.rs` if the endpoint dispatch changes.

## Current code

`Query::refine_partial` (`crates/before/src/causally/query.rs:220-252`) runs
only when the fused walk `filter::coverage` returned `Partial`. It
materializes both clamped endpoints, `lo | floor` and `hi & ceiling`, compares
them, and then hands one of them to `filter::admits` against the holes:

    let clamped_lo = match self.floor { Some(f) => lo | f, None => lo };
    let clamped_hi = match self.ceiling { Some(c) => hi & c, None => hi };
    if clamped_lo <= clamped_hi { ... holes ... } else { Coverage::Empty }

## The equivalence

The walk returns `Partial` only after it has established, for the query's one
normalized floor `f` and one normalized ceiling `c`:

- `f <= hi` (otherwise the floor's arm returns `Empty`, `filter.rs:546-549`)
- `lo <= c` (otherwise the ceiling's arm returns `Empty`, `filter.rs:560-563`)

and every `Span` has `lo <= hi`. In a lattice,
`lo | f <= hi & c` holds iff `lo <= hi`, `lo <= c`, `f <= hi`, and `f <= c`
all hold. Under the walk's `Partial` the first three are known, so the clamp
is crossed exactly when both bounds exist and `f <= c` fails. Without a floor
or a ceiling the clamp is never crossed.

That is: after a `Partial` walk, whether the clamped segment is empty is a
property of the query alone, `!(floor <= ceiling)`.

## Proposed structure

1. Replace the clamp comparison with
   `let crossed = matches!((floor, ceiling), (Some(f), Some(c)) if !(f <= c));`
   returning `Empty` when crossed.
2. Materialize only the clamped endpoint the polarity's holes need, and only
   when holes exist: `hi & ceiling` (or `hi`) for `Down`, `lo | floor` (or
   `lo`) for `Up`. A `Neutral` query never materializes either.
3. State the coupling at both sites, per the crate's rule that a fragile
   invariant is documented where established and where relied on:
   `refine_partial`'s doc names the walk's two established inequalities, and
   `filter::coverage`'s doc says its `Partial` arm guarantees them.

## Measured effect (scan meter, debug build, ox-east-1)

Probe `l5_refine_partial_scan_cost` on `explore/l5-spans` @ `2874e0c3`
(neutral query `after(floor) & before(ceiling)`, verdict `Partial`, grids of
64 to 512 cells):

| cells | input bytes | total scan | walk scan | refine scan | `floor <= ceiling` scan |
|------:|------------:|-----------:|----------:|------------:|------------------------:|
|    64 |         102 |       2030 |       799 |        1231 |                     476 |
|   128 |         202 |       4062 |      1599 |        2463 |                     956 |
|   256 |         402 |       8126 |      3199 |        4927 |                    1916 |
|   512 |         802 |      16254 |      6399 |        9855 |                    3836 |

Every column grows linearly, so this is a constant factor: the refinement
costs about 1.54 times the walk. The reduction would bring the neutral
`Partial` call from 2030 to about 1275 bits at 64 cells (a 37% cut), and it
also removes one or two heap-allocated lattice results. These readings cover
the neutral case only; for `Down` and `Up` the saving is the one unneeded
clamp (inferred from the structure, not measured).

## Why the result is more obviously correct, and the trade

The new code states the emptiness condition in the query's own terms
("the clamp is crossed iff the floor is not below the ceiling"), which reads
more directly than an endpoint-derived comparison. The cost is a documented
cross-module dependency on the walk's `Partial` guarantees. If the owner
prefers `refine_partial` to stay self-contained, keep the current form and
record this as declined. That is a legitimate choice.

## Coverage that protects the change

- `coverage_is_exact_on_the_two_party_grid` and
  `coverage_clamp_refinement_is_exact` (`causally/tests.rs`): the crossed
  clamp and the hole-covers-clamped-top cases.
- `public_queries_match_their_relational_denotation`: exact coverage over the
  two-party grid for random conjunctions.
- The L5 census properties (`l5_queries_match_vectors`,
  `l5_exhaustive_boolean_cube`) on the explore branch; mutants M4 and M5
  (wrong covering endpoint; skipped crossed check) were caught by both these
  and the committed suite.

## Owner's ruling

Accepted. `refine_partial` is private, and the coverage walk is its only
caller. It may rely on the walk's guarantees (`floor <= hi`, `lo <= ceiling`)
and decide by `floor <= ceiling`, stating the precondition in its docs and at
the call site.
