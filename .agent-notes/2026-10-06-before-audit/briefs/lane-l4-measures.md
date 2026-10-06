<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the lane section for the L4 auditor, modeled on the L6 codec lane; under review by Finch before launch. -->

# Lane L4: measures (Rank, Ranked, Count)

## Scope

This lane's theme is *measurement*: turning versions into exact numbers and
into a total order. `Rank` is the exact area under a version's step function;
`distance` and `lag` are differences of such areas; `Ranked` extends the causal
order to a total order; `Count` is the exact count type. All of these are
arbitrary-precision, so exactness, normalization, and representation
boundaries are where defects hide.

Some things to consider, as starting points rather than a checklist. They
are prompts for your own exploration: pursue whatever else the code
suggests, and expect the most valuable questions to be ones not listed
here.

- Is every measure exactly the mathematical quantity it claims to be, at any
  magnitude? In particular, is rank exactly the area, and therefore modular?
- Do equal values built by different routes behave identically under every
  operation? The routes include arithmetic in different orders, subtraction,
  decoding, parsing, and measurement of a version.
- Do the byte encodings preserve order (lexicographic order equals numeric
  order, even with arbitrary suffixes appended) so that they can key a
  byte-ordered store?
- Are the text forms canonical, and do they round-trip?
- Do the arithmetic paths keep their cost claims for every operand order and
  magnitude?

The identifiers below are a starting map, not a boundary. Follow the theme
wherever the code takes it, including into code this list does not name.

- **Version measures:** `rank`, `ranked`, `distance`, `lag`, `encode_rank`.
- **`Rank`:** comparison, hashing, addition, subtraction, `Sum`, text, and the
  ordering semantics of its encoding.
- **`Ranked`:** comparison, hashing, `rank`, `version`, `into_owned`, and the
  ordering semantics of `encode` and `encode_rank`.
- **`Count`:** arithmetic, `Sum`, `limbs`, text, primitive conversions.
- **Implementation:** `src/rank.rs`, `src/ranked.rs`, `src/count.rs`,
  `src/version/measure/`, `src/accumulator.rs`.

Neighboring lanes explore adjacent themes. When your work crosses into one,
establish what you found and report it, rather than continuing the
investigation there. L6 explores the codec mechanics, and L7 the `suanpan`
accumulator beneath the arithmetic, including whether `before`'s calls into it
respect its contract.

## Contracts to read first

- `Rank`'s type docs: the exact area under the version's event function;
  strictly monotone in causal order; lexicographic encoding order equals
  numeric order; and the encoding is self-delimiting, such that "an arbitrary
  suffix may be appended to each encoded rank without changing the order
  between distinct ranks".
- `Version::distance`: symmetric, zero only between equal versions, and
  obeying the triangle inequality.
- `Version::lag`: `a.lag(b) + b.lag(a) == a.distance(b)`.
- `Ranked`'s docs: ordered by rank, with the version's own bytes as the
  tiebreak.

## Prior coverage to map

- **Tests:** `src/version/measure/tests.rs` and its `integral` subtrees,
  `src/count/tests.rs`, `src/accumulator/tests.rs`, the rank law module,
  `tests/answer_embedded.rs`, `tests/meter/answer_embedded_product.rs`,
  `tests/meter/settle_flatness.rs`.
- **September partitions:** `rank.md`, `skyline-query.md`.

## Closed fixes to re-attack

- Rank sums linear in their combined content regardless of summand order.
- `Ranked::cmp`'s attainable worst case on the families that trigger
  settling.
- Rank-producing folds at their `O(M(n))` bound.
- `Rank`'s binary-point text, with formatter flags.

## Candidate leads (evaluate, don't confirm)

1. **Rank is the area, and the area is modular.** `rank` equals the
   function-space oracle's integral. `rank(a | b) + rank(a & b)` equals
   `rank(a) + rank(b)`, which the documented lag and distance identity
   requires.
2. **One value, one behavior.** Ranks equal in value but built by
   different routes (addition in different orders, subtraction, decoding,
   parsing, `Version::rank`) agree on `Eq`, `Hash`, `Ord`, `Display`, and
   `encode`.
3. **The suffix property.** For distinct ranks and arbitrary byte suffixes,
   the order between the encoded ranks plus their suffixes equals the ranks'
   order. The same holds for `Ranked`'s encoding and for
   `Version::encode_rank`.
4. **`Count` at the boundaries.** Conversions at every primitive's
   extremes, `limbs` exactness, and parse/display round trips.
